---
workstream: node-task-delete-dangling-shared-id
status: active
created: 2026-08-05
parent_session: vk-swarm-node-ui-localize post-close user report
---

# node-task-delete-dangling-shared-id

A node task whose `shared_task_id` points at a hive row that no longer exists is **permanently
undeletable**. Filed as **F-2026-08-05-01**.

## Symptom (user-reported, 2026-08-05)

Deleting a task in the node UI shows the confirm dialog, then fails with:

```text
{"error":"shared task not found"}
```

The task remains. There is no way to remove it from the UI.

## NOT caused by `vk-swarm-node-ui-localize`

That branch touches nothing on this path:

```text
$ git diff --name-only feff74be..HEAD -- \
    frontend/src/lib/api/tasks.ts frontend/src/hooks/useTaskMutations.ts \
    crates/server/src/routes/tasks/ crates/db/src/models/task/
(empty)
```

The error string itself originates in `crates/remote/` (the hive), which that branch leaves with a
verified-empty diff (SC7).

## Root cause

`crates/server/src/routes/tasks/handlers/core.rs:535-537` routes deletion to the hive whenever the
local task carries a `shared_task_id`:

```rust
// Check if this is a task synced from Hive (has shared_task_id)
if task.shared_task_id.is_some() {
    return delete_remote_task(&deployment, &task).await;
}
```

`delete_remote_task` (`crates/server/src/routes/tasks/handlers/remote.rs:226-231`) then does:

```rust
if let Some(shared_task_id) = task.shared_task_id {
    let remote_client = deployment.remote_client()?;
    let request = DeleteSharedTaskRequest { version: None };
    remote_client
        .delete_shared_task(shared_task_id, &request)
        .await?;          // <-- hive 404 propagates here and ABORTS the whole delete
```

The hive answers `404 {"error":"shared task not found"}`
(`crates/remote/src/routes/nodes.rs:1405-1412`, via `SharedTaskRepository::find_by_id`). The `?`
turns that into an `ApiError`, so the handler returns before any local cleanup and the row survives
every retry.

**The gap is a missing third case.** The function already handles two:

| `shared_task_id` | hive row | current behaviour |
|---|---|---|
| `None` | n/a | deletes locally (`remote.rs:240-249`) — correct |
| `Some(id)` | exists | delegates to hive; local cache cleaned by the `task.deleted` WS event |
| `Some(id)` | **missing (dangling)** | **aborts — task undeletable** |

A delete is idempotent: a 404 means the remote object is *already gone*, which is the desired end
state, not a failure. The dangling case should fall through to the same local deletion the `None`
branch performs.

## Likely origin of the dangling ids

`crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql` and
`crates/remote/tests/hive_cutover_migration.rs:34` `TRUNCATE` hive tables. Any node that synced
before a cutover keeps local rows referencing shared tasks the hive has since dropped. Worth
confirming against the live hive rather than assuming — count local tasks with a `shared_task_id`
that has no matching hive row.

## What "done" looks like

- A 404 / not-found from `delete_shared_task` falls through to local deletion instead of aborting.
- The stale `shared_task_id` is cleared (or the row deleted) so the state cannot recur.
- Genuine failures — auth, transport, 5xx, conflict — still propagate. **Do not** blanket-swallow
  errors from `delete_shared_task`; discriminate on not-found specifically. (This run has already
  produced five over-broad-predicate defects; an `is_err()` catch-all here would be the sixth.)
- A test drives the real seam with a dangling `shared_task_id` and asserts the task is gone —
  not a unit test of the helper.
- Consider the related hazard, separately: on the SUCCESS path the node deliberately does not delete
  locally, relying on the `task.deleted` WebSocket event (`remote.rs:238-239`). If that event is
  missed the task also lingers. Out of scope here; note it rather than fix it blind.
