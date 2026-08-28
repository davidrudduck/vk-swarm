# Code Review — Round 2

**Target:** remediation of round-1 actionable findings (working tree at HEAD `e51147c9`)   **Range:** `git diff` (uncommitted: `crates/server/src/routes/tasks/handlers/remote.rs`, `crates/server/tests/common/mod.rs`)   **Effort:** high

Verification pass over the three round-1 fixes (dead `HiveHarness::delete()` removal; unreachable else-branch removal; Ok-arm comment/log correction), plus regression hunt.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| — | — | — | — | none | — | — |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 1 | `crates/server/src/routes/tasks/handlers/remote.rs:220` | low | quality | With the else gone, a future caller passing `shared_task_id = None` would get a silent 202 no-op instead of a local delete; currently impossible (single guarded caller at `core.rs:596`; precondition documented in the fn doc). Optional `debug_assert!` polish. | high | Theoretical future-caller hazard; precondition documented. Logged to Post-review known issues. |

Verification detail: fix 1 CONFIRMED (zero consumers of removed method; only `.delete(` left is inside surviving `delete_with`); fix 2 CONFIRMED (caller guard `core.rs:596` holds; removal control-flow equivalent; reworded 404-arm comment accurate — hard `Task::delete` at `remote.rs:252`, `is_not_found()` discrimination intact); fix 3 CONFIRMED (`process_task_deleted_event` → `Task::unlink_by_shared_task_id` = `UPDATE tasks SET shared_task_id = NULL`, `sync.rs:495-507`, row retained proven by `unlink_by_shared_task_id_keeps_local_row`; no code asserts old or new log strings). Gates: fmt clean, `clippy -p server --tests --all-features -D warnings` exit 0, `tasks_delete_routes` 3/3 passed.

## Verdict: Approve

Actionable: []
