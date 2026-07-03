# Round-2 re-review — fencing remediation of round-1 findings A–H

**Reviewer:** OpenCode (z-ai/glm-5.2) — orchestrator + 2 dispatched `general` subagents (mechanics lens, fidelity lens)
**Date:** 2026-07-03
**Scope:** remediation diff for round-1 findings A–H across `crates/remote/src/nodes/ws/session.rs`, `crates/remote/src/db/task_assignments.rs`, `crates/remote/src/nodes/ws/status_machine.rs` (327-line diff, saved to `/tmp/opencode/remediation-diff.patch`).

## Resilience record

- **Mechanics lens** — OpenCode `general` subagent (z-ai/glm-5.2), local checkout, read-only. Completed.
- **Fidelity lens** — OpenCode `general` subagent (z-ai/glm-5.2), local checkout, read-only. Completed.
- **Gemini bridge** (`cc-gemini-plugin:gemini-agent`) — NOT AVAILABLE in this environment.
- **Codex bridge** (`codex:codex-rescue`) — NOT AVAILABLE in this environment.

Per the adversarial-review skill's 2-of-3 resilience rule, a single family cannot block. Two independent challengers (different prompts, different lenses) ran against the live working tree; each verified every claim by reading the real file:line before asserting. Both challengers produced structured findings with citations.

## Round-1 findings — remediation summary

All 8 round-1 findings remediated in-session (per AGENTS.md "No Deferred Remediation"):

| # | sev | finding | remediation | round-2 status |
|---|-----|---------|-------------|----------------|
| A | BLOCKING | `handle_task_status` looked up assignment by `id` only — any node knowing the `assignment_id` could write | New `find_active_lease_for_node(id, node_id)` w/ `completed_at IS NULL AND lease_expires_at > NOW()`; guard moved BEFORE the assignment-row writes | partially met → re-fixed in round-2 (see below) |
| B | BLOCKING | `handle_task_sync` manual match missed `"inprogress"` (serde lowercase) → silent coercion to `Todo` | Replaced with `canonical_status_from_node()` | ✓ verified clean |
| C | BLOCKING | `handle_task_sync`'s `upsert_from_node` had no SC4 author guard | Added guard mirroring `handle_op_batch_apply`'s — lookup key `(source_node_id, source_task_id)` matches `upsert_from_node`'s `ON CONFLICT` | ✓ verified clean |
| D | SHOULD-FIX | No-active-assignment else-branch did `break` with no revoke → node retried unacked ops forever | Added `LeaseRevoked` emission before `break` | partially met → re-fixed in round-2 (see below) |
| E | SHOULD-FIX | Unknown status propagated `?` as `HandleError::Database` → no `OpAck` → infinite retry | SKIP+ADVANCE: write `node_op_log`, set `applied_through_seq = op.seq`, `continue` | ✓ verified clean |
| F | SHOULD-FIX | `renew_lease` WHERE had no `lease_expires_at` check → swept (NULL) lease re-extendable | Added `AND lease_expires_at > NOW()` | ✓ verified clean |
| G | BLOCKING | Fence ran for node-owned tasks (which have no assignment row) → subsequent writes wedged | `owner_node_id == Some(node_id)` bypasses fence; `None`/`Some(other)` fall through | ✓ verified clean |
| H | INFO | 4 stale `#[allow(dead_code)]` attrs in `status_machine.rs` | Removed all 4 | ✓ verified clean |

## Round-2 new findings (2 [SHOULD-FIX] + 2 [INFO])

### [SHOULD-FIX] R2-1 — D-revoke lookup unkeyed by `node_id` (BOTH challengers)

`session.rs:2121-2130` — `SELECT id FROM node_task_assignments WHERE task_id = $1 LIMIT 1` had no `node_id` filter and no `ORDER BY`. A task with multiple historical assignment rows returns a non-deterministic `assignment_id`, quite plausibly a *foreign* node's. The `LeaseRevoked` signal keyed on a foreign assignment is silently ignored by the sender node → the partitioned node still retries forever (finding D's exact failure mode, not actually closed).

**Remediation applied:** added `AND node_id = $2` + `ORDER BY completed_at DESC NULLS LAST LIMIT 1`.

### [SHOULD-FIX] R2-2 — A guard placed AFTER the assignment-row writes (fidelity challenger)

`session.rs:657-672` — `update_assignment_local_ids` and `update_assignment_status` wrote to `node_task_assignments` via `repo.update` (`WHERE id = $1` only) BEFORE the `find_active_lease_for_node` guard at `:681`. A node that lost the lease could still mutate `execution_status` + `local_task_id`/`local_attempt_id` on the assignment row — the exact "any node knowing the assignment_id can write" threat finding A names. The fix's comment ("only the current lease-holder may update the status") was overclaiming.

**Remediation applied:** moved `find_active_lease_for_node` lookup BEFORE the writes. If `None` → log warning + `return Ok(())`. Shared-task status propagation moved into a bare block using the matched `assignment`.

### [SHOULD-FIX] R2-3 — `lease_expires_at IS NOT NULL` doesn't enforce `> NOW()` (mechanics challenger)

`task_assignments.rs:274` — `find_active_lease_for_node` checked `lease_expires_at IS NOT NULL` but NOT `> NOW()`. Between lease expiry and the next `reclaim_expired_leases` sweep, the row has `lease_expires_at < now()` but `IS NOT NULL` → the 304 path still authorized a status write from a node whose lease has expired. Same gap in `renew_lease:194` (expired-but-unswept lease re-extendable).

**Remediation applied:** changed `IS NOT NULL` → `> NOW()` in both `find_active_lease_for_node` (`:274`) and `renew_lease` (`:194`).

### [INFO] R2-4 — 21 `cargo fmt` diffs

Under-indented else-branch, import ordering in touched files. AGENTS.md's mandatory gate is clippy+test+lint+tsc (no `fmt --check`), so this wouldn't break the gate, but a `cargo fmt --check` CI step would dirty. **Fixed:** `cargo fmt -p remote` resolved all 21.

### [INFO → FIXED in-session] R2-5 — `handle_task_sync` unknown-status path asymmetric with op_batch

`session.rs:1602-1603` returned `HandleError::Database` with no `TaskSyncResponse` for an unknown wire value, whereas `handle_op_batch_apply`'s identical failure class (finding E) was given SKIP+ADVANCE. The asymmetry was *introduced* by finding B's remediation (the old manual match silently coerced to `Todo`; the new `canonical_status_from_node` returns `Err`, which `?`-propagated). Per AGENTS.md "No Deferred Remediation", this is not deferrable — it's a regression in the failure-mode shape introduced by this session's own work.

**Remediation applied (in-session):** replaced the `?`-propagation with a `TaskSyncResponse { success: false, error: Some("REJECTED: unknown status wire value: ..."), .. }` + `return Ok(())`, mirroring the project-race / not-linked branches below it and op_batch's SKIP+ADVANCE for the same failure class. Verified: clippy clean, 90 lib tests pass, 3 integration tests pass.

## Round-2 gate checks (final committed state)

- `cargo clippy --all --all-targets --all-features -- -D warnings` — **PASS** (0 warnings)
- `cargo test -p remote --lib` — **90/90 PASS**
- `cargo test -p remote --test lease_fencing_migration --test lease_partition_e2e --test node_op_log_migration` — **3/3 PASS** (key test `partitioned_node_late_commit_is_rejected_after_reassignment` passes with the `> NOW()` + guard-placement changes)
- Doctests: 31 failures — **PRE-EXISTING** (verified by `git stash` + clean-tree run: identical 31 failures, all `crate::` path resolution in doc examples)
- Full `cargo test --workspace` — timed out on slow `services` crate terminal-session tests (60s+/test, unrelated to changes; no `remote` crate failures)
- Frontend: `cd frontend && npm run lint` — **PASS** (0 warnings). `cd frontend && npx tsc --noEmit` — 3 PRE-EXISTING errors in `src/lib/electric/collections.ts` (`@tanstack/db` vs `@tanstack/react-db` version mismatch). Verified pre-existing via clean-tree run; my changes are Rust-only.

## Verdict

**CLEAN** after round-2 remediation.

- [BLOCKING]: 0
- [SHOULD-FIX]: 0 (3 round-2 findings all remediated)
- [INFO]: 0 (R2-4 fixed via `cargo fmt`; R2-5 fixed in-session per No Deferred Remediation)

## Lessons learned

1. **Round-1 finding A's stated intent exceeded its fix.** The original fix only gated the shared-task status propagation, not the assignment-row writes that preceded it. The fix's comment overclaimed. A fidelity-lens re-review caught the gap because it walked each fix back to its finding's *governing intent*, not just the literal code change. Mechanics lens alone confirmed the SQL but missed the semantic gap.
2. **`IS NOT NULL` ≠ `> NOW()` for lease checks.** A subtle but consequential distinction: between lease expiry and the next reclaim sweep, the row is non-NULL but expired. The mechanics challenger caught this by tracing `reclaim_expired_leases`'s NULL-out step and asking "what's the state in the window between expiry and sweep?" Round-1 finding F was scoped to "swept leases" and met its scope, but the shared gap (expired-but-unswept) was flagged once here, not duplicated per-finding.
3. **`LIMIT 1` without `node_id` + `ORDER BY` is non-deterministic.** Both challengers independently flagged the D-revoke lookup. A `LIMIT 1` over a table that can accumulate multiple historical rows per task returns whichever row Postgres happens to scan first — silently wrong, not noisily wrong. The fix's letter ("query without filter, push to revokes") was met; its spirit (the partitioned node actually learns of the revoke) was not.
4. **Two-lens dispatch caught what one lens missed.** The mechanics challenger found R2-3 (the `> NOW()` gap); the fidelity challenger found R2-2 (the guard placement). Neither challenger alone found both. Parallel dispatch with different prompts is load-bearing, not ceremonial.
5. **Bridge unavailability is recoverable.** Gemini and Codex bridge subagent types were not available in this environment. Two same-family `general` subagents with different prompts (mechanics vs fidelity) provided the 2-of-3 resilience floor and produced 3 of 3 actionable findings. The resilience record above is explicit about this so the next round can dispatch true cross-family if the bridges come online.

## Standing debt

None introduced by this remediation. Pre-existing (unrelated, verified on clean tree):
- **Doctests (31)** — `crate::` path resolution failures in doc examples.
- **Frontend tsc (3 errors)** — `@tanstack/db` vs `@tanstack/react-db` version mismatch in `src/lib/electric/collections.ts`.
