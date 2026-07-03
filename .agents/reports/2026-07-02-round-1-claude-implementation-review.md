The findings report is saved to the plan file. Here's the full adversarial review:

---

## Adversarial Tournament Report — hive-redesign Phases 1–3

**Scope:** 7 findings against the P1–P3 implementation. P4–P7 gaps (SC7/SC5/SC1/SC6) are expected and pre-documented in the decisions ledger.

---

### Finding 1: [BLOCKING] `handle_task_sync` silently maps `inprogress`/`inreview` → `Todo`

- **Location:** `crates/remote/src/nodes/ws/session.rs:1592-1599`
- **Evidence:** The match arm `"in_progress" | "in-progress"` never matches the node's actual wire value `"inprogress"` (serde `rename_all = "lowercase"`). Falls through to `_ => TaskStatus::Todo`. Confirmed: `hive_sync.rs:372` still emits `NodeMessage::TaskSync`. Task 302 fixed this in `handle_op_batch` via `canonical_status_from_node`; `handle_task_sync` was not updated.
- **Impact:** Every `TaskSync` for an in-progress or in-review task silently writes `todo` to `shared_tasks`. Systematic hive state corruption while the TaskSync path remains active.
- **Remediation:** Replace the `match` with `canonical_status_from_node(&task_sync.status).unwrap_or(TaskStatus::Todo)`.

---

### Finding 2: [BLOCKING] `handle_task_sync` calls `upsert_from_node` without SC4 author guard

- **Location:** `crates/remote/src/nodes/ws/session.rs:1707-1726`
- **Evidence:** `repo.upsert_from_node(...)` is called unconditionally with no `author_of_transition` / `node_may_author` check. The SC4 guard was applied to `handle_op_batch` (303) and `handle_task_status` (304) but not to this third write site.
- **Impact:** Any node can write hive-only transitions (`todo→done`, `in_review→done`) or illegal ones via `TaskSync`, bypassing ADR-0010 entirely.
- **Remediation:** Mirror the `handle_task_status` guard pattern (lines 690-720) before calling `upsert_from_node` in `handle_task_sync`.

---

### Finding 3: [SHOULD-FIX] `handle_task_status` (task 304) missing lease validity and node ownership checks

- **Location:** `crates/remote/src/nodes/ws/session.rs:677`, `crates/remote/src/db/task_assignments.rs:216-232`
- **Evidence:** `find_by_id` WHERE clause is `WHERE id = $1` only — no `AND node_id`, `AND completed_at IS NULL`, or `AND lease_expires_at > now()`. Task 304 spec explicitly required "gate on assignment being active (lease not expired)". None of the three conditions are present.
- **Impact:** A partitioned node whose lease was swept can send `TaskStatus` for an assignment now held by node B, pass `node_may_author(InProgress, InReview) = true`, and overwrite the task — bypassing P2 fencing on the legacy path.
- **Remediation:** Add a `find_active_for_node(assignment_id, node_id)` method with `AND node_id = $2 AND completed_at IS NULL AND (lease_expires_at IS NULL OR lease_expires_at > now())` and use it in place of `find_by_id`.

---

### Finding 4: [SHOULD-FIX] No `LeaseRevoked` emitted in "no active assignment" else-branch; comment is wrong

- **Location:** `crates/remote/src/nodes/ws/session.rs:2007-2021`
- **Evidence:** The stale-token path at lines 1990-2005 calls `revokes.push(...)`. The else-branch (no active assignment) at line 2007 just `break`s. Comment at line 2012 says "the reclaim sweep's LeaseRevoked event will halt the node" — but `lease_sweep.rs` has no WS sender access and emits zero WS messages.
- **Impact:** Node receives an `OpAck` only up to the previous seq, then retries the unacked op indefinitely (~every 30s). No halt signal is ever delivered.
- **Remediation:** Add `revokes.push((assignment_id, "no active assignment".to_string()));` before the `break` in the else-branch. Also add `assert_eq!(revokes.len(), 1)` in the test at line 3522 that currently discards `_revokes`.

---

### Finding 5: [SHOULD-FIX] Unknown/absent status in op payload propagates `?`, wedging batch permanently

- **Location:** `crates/remote/src/nodes/ws/session.rs:2027-2033`
- **Evidence:**
  ```rust
  let status_raw = op.payload.get("status").and_then(...).unwrap_or("");
  let status = canonical_status_from_node(status_raw).map_err(|e| HandleError::Database(...))?;
  ```
  The `?` propagates through `handle_op_batch_apply` → `handle_op_batch` line 2197 → caught at line 237 with `tracing::warn!`. No `OpAck` sent. Node retries forever.
- **Impact:** Any op with missing or unrecognized `status` field permanently halts op-log processing for that node. Future op types without a `status` field would trigger this.
- **Remediation:** Treat unknown status as SKIP+ADVANCE (`record_in_op_log + continue`) rather than propagating an error.

---

### Finding 6: [SHOULD-FIX] `renew_lease` re-extends swept (NULL expiry) leases — sweep is bypassable

- **Location:** `crates/remote/src/db/task_assignments.rs:187-195`
- **Evidence:** `reclaim_expired_leases` sets `lease_expires_at = NULL` but leaves `node_id` unchanged. `renew_lease` WHERE clause is `WHERE id = $1 AND node_id = $2 AND completed_at IS NULL` — no `AND lease_expires_at IS NOT NULL`. Original holder reconnects, heartbeats, and `renew_lease` matches the swept row, re-extending the lease with the already-bumped T2 token.
- **Impact:** `reclaim_expired_leases` is ineffective against reconnecting original holders when no new `try_claim` has run. ADR-0009 bounded-overlap execution is violated.
- **Remediation:** Add `AND lease_expires_at IS NOT NULL` to `renew_lease`'s WHERE clause.

---

### Finding 7: [INFO] Stale `#[allow(dead_code)]` on all four items in `status_machine.rs`

- **Location:** `crates/remote/src/nodes/ws/status_machine.rs:13, 28, 48, 61`
- **Evidence:** All four items are now consumed by tasks 303/304. The attributes were task-301 scaffolding ("add until 303 wires the call site"). They suppress legitimate dead-code detection.
- **Remediation:** Remove all four `#[allow(dead_code)]` attributes.

---

```
VERDICT: REVISE
TOTAL FINDINGS: 7 (2 [BLOCKING], 4 [SHOULD-FIX], 1 [INFO])
```

The two BLOCKING findings share a common root: `handle_task_sync` was not updated when task 302 introduced `canonical_status_from_node` and task 303 introduced the SC4 author guard. Every other P1–P3 implementation detail was verified correct per spec.