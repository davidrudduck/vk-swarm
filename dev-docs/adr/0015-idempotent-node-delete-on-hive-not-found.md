# ADR-0015: Idempotent node task delete — hive not-found falls through to local row deletion

- Status: accepted
- Date: 2026-08-05
- Workstream: node-task-delete-dangling-shared-id (F-2026-08-05-01)

## Context

`delete_remote_task` (`crates/server/src/routes/tasks/handlers/remote.rs`) propagates the hive's
404 ("shared task not found") with `?`, aborting before local cleanup. A node task whose
`shared_task_id` points at a hive row that no longer exists (e.g. after the hive cutover
truncations) is therefore permanently undeletable.

## Decision

When `delete_shared_task` fails with **not-found specifically**
(`RemoteClientError::is_not_found()`, pinned to `Http { status: 404 }`), the node deletes the
local task row outright — the same `Task::delete` the `shared_task_id: None` branch performs —
rather than clearing the stale id and re-dispatching. Deleting the row destroys local task data
(title, description, attempts cascade), which is irreversible; it is nevertheless correct because
the user explicitly requested the delete and the remote object is already gone, so local deletion
is the desired end state of an idempotent delete.

All other errors (auth, transport, timeout, 5xx, 409 conflict) continue to abort the delete and
surface an error. No blanket `is_err()` handling — discrimination is on the 404 status only.

## Consequences

- Dangling tasks become deletable on demand; the stale `shared_task_id` disappears with the row.
- Hive behaviour is untouched; the 404 response remains correct.
- The success path (hive row exists) stays delegation-only via the `task.deleted` WS event; the
  known missed-WS-event hazard is out of scope here.
