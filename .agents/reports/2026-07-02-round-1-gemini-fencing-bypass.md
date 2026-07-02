# Adversarial Review Round 1 — Gemini Panelist

**Date:** 2026-07-02
**Target:** Branch `worktree-bridge-cse_01B8B52jjMEdikaLRoA25qhr` HEAD `9c4f9acd` vs `origin/main` — Phase 1 (op-log, SC2) + Phase 2 (lease/fencing, SC3) implementation.
**Governing intent:** `docs/superpowers/specs/2026-06-26-vk-swarm-hive-redesign.md` (SC2 + SC3).
**Panelist:** Gemini CLI (codebase-review mode, read-only).
**Question answered:** Does the implementation meet the intended goals (SC2 and SC3)?

## Resilience record

- **Gemini:** ran via `gemini-cli-panel` wrapper (`codebase-review` task, read-only). Local checkout, cwd = worktree root.
- **Opus / Codex:** not dispatched this round — user requested Gemini-only.

## Consolidation summary

| # | Issue | Tag | Accepted? | Impact if shipped | Remediation |
|---|---|---|---|---|---|
| F1 | Non-transactional outbox enqueue breaks SC2c no-loss | [BLOCKING] | **NO — legitimate scope split** | Node crash between task write and outbox enqueue loses the op | None — RATIFIED tracer limitation (ledger L131-136), legacy sync is backstop, transactional enqueue is the planned next Phase-1 increment |
| F2 | Fencing guard bypasses completed tasks, allowing double execution | [BLOCKING] | **YES — REAL BUG** | Partitioned node's late commit overwrites a completed task (SC3 violation) | Reject the op when `shared_id` is present but no active assignment exists |

## F1 — Adjudication: NOT ACCEPTED (legitimate scope split)

**Gemini's claim:** `enqueue_task_upsert_op` is called outside the same transaction as `Task::create`, so a crash between the two statements silently loses the outbox op (SC2c violation). Concurrent creations can break parent-before-child ordering (SC2b).

**Adjudication — DISMISSED with evidence:**

The decisions-ledger (`docs/plans/vk-swarm-hive-redesign/decisions-ledger.md:131-136`) explicitly ratifies this as a **TRACER LIMITATION**, not a bug:

> 4. **105 enqueue is best-effort, non-atomic** with the task write (callers hold `&SqlitePool`, not a txn) — RATIFIED as a TRACER LIMITATION. The legacy sync path is the backstop. **Consequence: Phase-1-tracer does NOT fully satisfy SC2c "zero silent write loss" — true no-loss needs the enqueue in the SAME transaction as the entity write** (threading a txn through the ~8 `Task::create` callers), which is the next Phase-1 increment.

This satisfies the AGENTS.md legitimate-scope-split exception (all 3 requirements):
1. **Explicitly named:** "transactional enqueue (full SC2c no-loss)" — the next Phase-1 increment.
2. **Tracked follow-up:** plan.md Phase-1 section + ledger L145-147 ("NOT yet done: transactional enqueue").
3. **Documented in decisions-ledger before the PR was submitted** (committed at `557bd5a2`, the Phase-1 reachability gate, before any push).

The code comment at `queries.rs:334-338` documents the same: "a failed enqueue is logged, NOT propagated — the legacy path remains the backstop." The legacy `hive_sync` path runs ALONGSIDE the new op-log (additive, not replacement), so a lost outbox op is caught by the legacy sync. SC2c is claimed by the durable-ack mechanism (102/106/108), not by the enqueue — the transactional enqueue is the closing increment.

**Parent-before-child ordering (SC2b) is NOT violated:** ordering is enforced by the hive's `handle_op_batch_apply` parking logic (106's third test), not by outbox seq ordering. A child op arriving before its parent's shared_task_id exists parks until the parent is linked.

**Verdict:** This is a planned scope increment, not deferred remediation from a review finding. No fix applied.

## F2 — Adjudication: ACCEPTED (REAL BUG, fixed in-session)

**Gemini's claim:** The fencing guard queries `WHERE task_id = $1 AND completed_at IS NULL`. For a completed/cancelled task with no active assignment, the query returns `None`, and the code falls through to normal apply — allowing a partitioned node's late write to overwrite the completed task.

**Adjudication — CONFIRMED by code trace:**

Trace (real merged code):
1. `crates/remote/src/nodes/ws/session.rs:1946` — `if let Some(shared_id) = shared_id {`
2. `session.rs:1949-1959` — query `SELECT id, fencing_token FROM node_task_assignments WHERE task_id = $1 AND completed_at IS NULL`
3. `session.rs:1961` — `if let Some((assignment_id, current_token)) = assignment {` → stale check
4. `session.rs:1984-1986` — `// No active assignment → fall through to 106's normal apply (node-owned or unassigned work; the fence does not apply).`

**SC3 bypass scenario (verified against `task_assignments.rs`):**
- Task T assigned to A (token T1). A partitioned.
- `reclaim_expired_leases` (`task_assignments.rs:502-526`) → UPDATE A's row: `fencing_token=T_reclaim`, `lease_expires_at=NULL`. `completed_at` stays NULL.
- B calls `try_claim` (`task_assignments.rs:110-128`) → UPDATE branch matches (`completed_at IS NULL`, `lease_expires_at IS NULL`) → sets `node_id=B`, `fencing_token=T2`.
- B completes the task → `completed_at` set (e.g. `task_assignments.rs:480-484`).
- A's late op (token T1) arrives → guard query returns **None** (the only row has `completed_at` set).
- Falls through to normal apply → **A overwrites the completed task. SC3 violated.**

**The root cause:** the fall-through at line 1984 assumes "no active assignment = node-owned or unassigned work" — but if `payload.shared_task_id` is present, it IS a hive-managed task. No active assignment means the lease was reclaimed or completed. A late write from a partitioned node must be rejected.

**Fix applied:** In `session.rs`, when `shared_id` is present and `assignment` is `None`, reject the op (break, don't apply) instead of falling through to normal apply. This closes the SC3 bypass: a partitioned node's late write to a completed/reclaimed task is dropped, not applied.

## Verdict (post-remediation)

- SC2 met: **PARTIAL** (by design — tracer limitation, transactional enqueue is the next Phase-1 increment; legacy sync is the backstop)
- SC3 met: **YES** (post-fix — the fencing guard now rejects ops for hive-assigned tasks with no active assignment, closing the completed-task bypass)
- Overall: **PASS** (F2 remediated; F1 is a legitimate scope split, not debt)

## Lessons learned

A single-model review (Gemini) caught a real SC3 bypass that the per-task Stage-2 panels missed — because the per-task panels review one task at a time, while the bug is a **cross-task interaction**: the fencing guard (task 205) + the reclaim logic (task 209) + the completion path (pre-existing `fail_node_assignments`) combine to produce a query that returns None at the wrong time. The per-task panels correctly verified each task in isolation; the integrated review caught the interaction. This validates the value of a post-phase integrated adversarial review alongside per-task panels.

The false positive (F1) shows the importance of adjudicating every finding against the decisions-ledger before accepting — a finding that looks like a bug may be a documented, ratified scope split.
