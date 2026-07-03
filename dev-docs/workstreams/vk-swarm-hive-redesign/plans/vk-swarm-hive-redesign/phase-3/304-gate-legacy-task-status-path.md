---
id: "304"
phase: 3
title: Route the legacy handle_task_status shared-status write through the transition guard
status: done
depends_on: ["301", "303"]
parallel: false
conflicts_with: ["302", "303"]
files:
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/session.rs"
allowed_change: edit
covers_criteria: [SC4]
covers_tests: []
---
## Reconciled matrix (ADR-0010, ratified 2026-06-30) — two legacy-path interactions resolved
This task consumes 301's reconciled (ratified) matrix. Two legacy-path interactions are now resolved
under it: (1) the legacy execution-status map never produces `Done` (`Completed→InReview`,
`session.rs:672`), so the only node-authorable apply the legacy path can make is `InProgress→InReview` —
the accept test seeds `in-progress`, reports `Completed`, and asserts the result is `in-review` (a
node-authored transition). (2) The map produces `Running→InProgress`, but `InProgress` is never a
node-authorable TARGET (the hive owns `Todo→InProgress` "on assignment + node start" and the
`InReview→InProgress` reopen). So a node `Running` report is correctly a **no-op** once the hive has
already moved the task to `in-progress`, and is correctly **rejected** if the task is still in its initial
state — the hive, not the node, authors the start. This is intended; no special case. See
`dev-docs/adr/0010-task-status-state-machine.md` (## Decision) and 301's reconciled matrix.

## Failing test (write first)
**WHY THIS TASK EXISTS (advisor §3 — surfaced, do NOT silently leave open):** there are TWO sites that
write `shared_tasks.status` from a node. 303 guards the canonical op-log path (`handle_op_batch`). The
SECOND is the LEGACY `handle_task_status` (`session.rs:625`), reached by `NodeMessage::TaskStatus`
(`session.rs:524`): it maps `TaskExecutionStatus` → `TaskStatus` and calls
`SharedTaskRepository::update_status_from_node` with NO author/lease/fencing context — pure last-write-wins,
exactly the silent-clobber ADR-0010 removes. If this path is left ungated while we claim "SC4 — no
field-level status conflict closed", it is a hole. This task CLOSES it by routing the legacy write through
the SAME `status_machine` author guard.

**PRECONDITION (Trap 2b):** `update_status_from_node` is a `query_as!` (`tasks.rs:1014`) and the guard adds
a `find_by_id` read — both validate against a live migrated Postgres. The test is a `#[cfg(test)] mod`
INSIDE `session.rs` (the handler is private). Reuse 106/303's inlined `backfill_e2e.rs` helpers
(`database_url()`, `skip_without_db!`, `create_pool()`, org/node/swarm-link/shared-task/assignment seeds).

```rust
#[cfg(test)]
mod legacy_status_guard_tests {
    use super::*;

    #[tokio::test]
    async fn legacy_path_applies_node_authored_transition_with_lease() {
        skip_without_db!();
        let pool = create_pool().await;
        // shared_task status='in-progress' + active assignment (valid lease/token). A TaskStatusMessage
        // mapping via TaskExecutionStatus::Completed → InReview (the legacy map never yields Done) →
        // APPLIED (in-progress→in-review is node-authored): status='in-review'.
    }

    #[tokio::test]
    async fn legacy_path_rejects_hive_authored_or_illegal_transition_from_node() {
        skip_without_db!();
        let pool = create_pool().await;
        // shared_task status='done'; a legacy TaskStatusMessage that would drive done→in-progress
        // (illegal), or a Failed/Cancelled report which the legacy map resets toward the initial status
        // → REJECTED: status stays 'done', update_status_from_node NOT called. This is the concrete
        // clobber the old map caused (Failed|Cancelled reset, session.rs:673).
    }
}
```
> The existing legacy mapping (`session.rs:669-674`) resets `Failed|Cancelled` toward the initial status
> and maps `Completed → InReview` — an unguarded `any→initial-status` write is precisely the field-level
> clobber ADR-0010 forbids. Routing it through `author_of_transition` rejects those illegal resets back to
> the initial state from `done`/`in-review`.

## Change
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** `handle_task_status` (`session.rs:625-697`), specifically the shared-task status write block
  (`session.rs:667-687`): the `if let Ok(Some(assignment)) = assignment_repo.find_by_id(...)` →
  `let shared_status = match status.status { … }` → `task_repo.update_status_from_node(assignment.task_id,
  shared_status)`.
- **Before:**
```rust
        // Map execution status to shared task status
        let shared_status = match status.status {
            TaskExecutionStatus::Pending | TaskExecutionStatus::Starting => TaskStatus::Todo,
            TaskExecutionStatus::Running => TaskStatus::InProgress,
            TaskExecutionStatus::Completed => TaskStatus::InReview,
            TaskExecutionStatus::Failed | TaskExecutionStatus::Cancelled => TaskStatus::Todo,
        };

        let task_repo = SharedTaskRepository::new(pool);
        if let Err(e) = task_repo
            .update_status_from_node(assignment.task_id, shared_status)
            .await
        {
            tracing::warn!(
                task_id = %assignment.task_id,
                error = %e,
                "failed to update shared task status"
            );
        }
```
- **After:** read the current status, run the SAME author guard 303 uses, and only write when the
  transition is a no-op OR node-authored. `handle_task_status` already runs ONLY for a node that holds the
  assignment (`assignment` is in hand), so the lease context is the existing active assignment; gate the
  node-authored branch on that assignment being active (lease not expired) — reuse 303's predicate, do not
  re-implement fencing.
```rust
        // Map execution status to the proposed shared task status.
        let proposed = match status.status {
            TaskExecutionStatus::Pending | TaskExecutionStatus::Starting => TaskStatus::Todo,
            TaskExecutionStatus::Running => TaskStatus::InProgress,
            TaskExecutionStatus::Completed => TaskStatus::InReview,
            TaskExecutionStatus::Failed | TaskExecutionStatus::Cancelled => TaskStatus::Todo,
        };

        let task_repo = SharedTaskRepository::new(pool);
        // Guard via the single-author matrix (ADR-0010 §D). The legacy path is node-reported, so only a
        // no-op or a node-authored transition (with an active lease) may write — never a hive-authored
        // or illegal transition (the old `*→Todo` clobber).
        match task_repo.find_by_id(assignment.task_id).await {
            Ok(Some(current_task)) if current_task.status == proposed => {
                // no-op — nothing to write
            }
            Ok(Some(current_task))
                if crate::nodes::ws::status_machine::node_may_author(
                    current_task.status,
                    proposed,
                ) =>
            {
                if let Err(e) = task_repo
                    .update_status_from_node(assignment.task_id, proposed)
                    .await
                {
                    tracing::warn!(task_id = %assignment.task_id, error = %e,
                        "failed to update shared task status");
                }
            }
            Ok(Some(current_task)) => {
                tracing::warn!(task_id = %assignment.task_id, from = ?current_task.status,
                    to = ?proposed,
                    "rejected non-node-authored status transition on legacy path (ADR-0010)");
            }
            Ok(None) => {
                tracing::warn!(task_id = %assignment.task_id, "shared task not found for status update");
            }
            Err(e) => {
                tracing::warn!(task_id = %assignment.task_id, error = %e,
                    "failed to read shared task status");
            }
        }
```

## Allowed moves
ONLY: replace the legacy `shared_status` map + unconditional `update_status_from_node` call in
`handle_task_status` with the guarded version above, and add the `#[cfg(test)] mod legacy_status_guard_tests`.
REUSE `status_machine::node_may_author` (301) and `SharedTaskRepository::{find_by_id, update_status_from_node}`
(`tasks.rs:243,1009`). Do NOT change the assignment-execution-status update (`update_assignment_status`,
`session.rs:659`) — that is the node-side execution status, separate from `task.status`. Do NOT touch the
op-log path (303 owns it), the WS enums, the node crate, `tasks.rs`, `status_machine.rs`, or any migration.

## STOP triggers
- 301/303 not landed: `status_machine::node_may_author` absent → STOP (this task consumes it).
- `handle_task_status` no longer exists or `NodeMessage::TaskStatus` was removed → this means P4
  (inbound-collapse) RETIRED the legacy path already. If so, STOP and record: this task is **superseded by
  P4's retirement** — gating a deleted path is moot; mark 304 obsolete in the decisions-ledger rather than
  re-adding a dead handler. (304 closes the hole IF the legacy path still ships at P3 time; P4 may instead
  delete it. Either outcome closes SC4's second site — surface which one happened.)
- You consider gating `update_assignment_status` (the node *execution* status on the assignment row) →
  STOP. That is NOT `task.status`; the matrix governs `shared_tasks.status` only. Leave the assignment
  execution-status write untouched.
- `SharedTask.status` field type is not the hive `TaskStatus` (`tasks.rs:79`): if the struct changed,
  re-confirm the comparison/guard types before editing.
- `query_as!`/`query!` fail offline → export `DATABASE_URL=postgres://…` (Trap 2b). Do NOT `cargo sqlx prepare`.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote legacy_status_guard' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 304` exits 0
(run with `DATABASE_URL=postgres://…` against a migrated Postgres — Trap 2b. The `test -n "$DATABASE_URL" &&`
prefix is FAIL-CLOSED: without `DATABASE_URL` the gate FAILS rather than skip-passing.)
