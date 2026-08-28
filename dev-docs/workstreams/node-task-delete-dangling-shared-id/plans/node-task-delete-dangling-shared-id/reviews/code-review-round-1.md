# Code Review — Round 1

**Target:** PR #468 merge commit `bd80dfa0` (node-task-delete-dangling-shared-id), verified against current HEAD `e51147c9`   **Range:** `bd80dfa0^..bd80dfa0`   **Effort:** high

Two parallel finder subagents (correctness/security lens + quality lens), verify-before-assert against the current tree.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `crates/server/tests/common/mod.rs:424-451` | low | quality | `HiveHarness::delete()` is dead code — added by bd80dfa0, zero callers at HEAD (all consumers use `delete_with` after the auth workstream); the blanket `#[allow(dead_code)]` on the impl block silenced the compiler. Also an unauthenticated DELETE (any reuse would 401). | high | yes |
| 2 | `crates/server/src/routes/tasks/handlers/remote.rs:258-267` | low | quality | The `else` (no-`shared_task_id`) branch in `delete_remote_task` is unreachable: the sole caller guards `task.shared_task_id.is_some()` (`core.rs:596`) and the fn doc says it is called only when a shared id exists — yet the 404-arm comment and ADR-0015 cite this dead branch as semantic precedent. | high | yes |
| 3 | `crates/server/src/routes/tasks/handlers/remote.rs:225,237,239-240` | low | quality | Ok-arm comment/log claim the WS handler will "clean up the local cache", but the `task.deleted` handler soft-unlinks and RETAINS the local row (`processor.rs:436-446`); misleading since TS3's row-retained assertion matches reality, not the comment. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 1 | `crates/server/src/routes/tasks/handlers/remote.rs:248` | low | correctness | `is_not_found()` matches any `Http{404}` from the hive base URL, so a gateway/route-absent 404 also triggers the irreversible local `Task::delete` cascade. | medium | ADR-0015 deliberately pins discrimination to status-only; body-sniffing would couple node to hive internals. Accepted residual risk (ledgered below). |
| 2 | `crates/server/src/routes/tasks/handlers/remote.rs:254` | low | correctness | The 404 fall-through's bare `Task::delete` skips the worktree cleanup the core delete path performs (`core.rs:611-635`), potentially orphaning on-disk attempt worktrees; FK `ON DELETE SET NULL` keeps DB consistent, ADR-0015 silent on disk state. | medium | Edge-case (dangling shared task WITH local attempts AND on-disk worktrees); runtime-deletion governance adjudicated under ADR-0015. Logged to Post-review known issues for a future workstream. |
| 3 | `crates/server/tests/common/mod.rs:455-471` | low | quality | `seed_shared_task` duplicates the `CreateTask`+`Task::create` block in `seed_project`'s loop; a parameterized seeder would serve both. | high | Cosmetic, harness-local duplication; churn not warranted at close. |
| 4 | `crates/server/tests/common/mod.rs` (multiple) | low | quality | `delete()` was the 8th verbatim copy of the ~26-line Resp-mapping block (`get`, `post`, `get_with`, …); a shared `into_resp` helper would collapse them. | high | Pre-existing pattern across the harness (out of this diff's scope); finding 1 removes the copy this diff added. |

Verified sound (no finding): 404 semantics end-to-end (hive 404 only for missing task; 401/403 collapse to Auth and abort; 404 never retried); `Err(e) => Err(e.into())` identical to prior `?`; concurrent duplicate 404-deletes safe (`DELETE ... RETURNING` in one tx); no authz change (tests exercise authenticated path); tests drive the REAL served router with wiremock fronting only the hive; delegation-only Ok path pinned by `tasks_delete_routes.rs:94-134`.

## Verdict: With fixes

Actionable: [1,2,3]
