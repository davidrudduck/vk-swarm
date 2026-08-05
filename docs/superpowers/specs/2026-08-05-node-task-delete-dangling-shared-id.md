---
doc_type: spec
status: active
workstream: node-task-delete-dangling-shared-id
change_kind: bugfix
verify_cmd: "grep -rqi 'dangling shared_task_id' ~/.local/share/vibe-kanban/logs 2>/dev/null || journalctl -u vks-node --since -7d 2>/dev/null | grep -qi 'dangling shared_task_id'"
---

# node-task-delete-dangling-shared-id

## Intent
Finding F-2026-08-05-01 (high, promoted). Full root-cause narrative: dev-docs/workstreams/node-task-delete-dangling-shared-id/README.md (linked, not duplicated).

A node task whose `shared_task_id` points at a hive row that no longer exists is permanently undeletable. `DELETE /api/tasks/:id` routes to the hive whenever `shared_task_id` is set (`crates/server/src/routes/tasks/handlers/core.rs:535-537`); `delete_remote_task` (`crates/server/src/routes/tasks/handlers/remote.rs:229-231`) propagates the hive's 404 (`{"error":"shared task not found"}`) with `?`, aborting before any local cleanup. The row survives every retry.

Delete is idempotent: a not-found from the hive means the remote object is already gone (the desired end state), so the handler must fall through to the same local deletion the `shared_task_id: None` branch performs. Genuine failures (auth, transport, 5xx, conflict) must still abort and surface an error: discriminate on not-found ONLY, never a blanket `is_err()` catch.


## User stories
- **US1:** As a node user, when I delete a task whose shared_task_id no longer exists on the hive, I expect the task to be deleted locally instead of the delete failing with 'shared task not found'.
- **US2:** As a node operator, when the hive delete fails for a real reason (auth failure, transport error, 5xx, version conflict), I expect the delete to abort with a surfaced error so I never silently lose sync integrity.

## Success criteria
SC1: On DELETE /api/tasks/:id for a task with a dangling shared_task_id (hive answers 404), the node's HTTP response is 202 success and the task row is absent from the node DB afterwards -- the hive's not-found no longer aborts local cleanup.
→ US1
SC2: On DELETE /api/tasks/:id for a shared task where the hive responds with a non-not-found error (e.g. 409 conflict), the node responds with an error status and the local task row still exists (delete aborted).
→ US2
SC3: The not-found fall-through emits a tracing warn line naming the dangling shared_task_id, observable in the running node's logs, so operators can see dangling rows being reaped.
→ US1

## Users
- Node operators / UI users: cannot delete tasks left dangling by hive cutover truncations (`crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql`).
- Any node that synced before a hive data reset.


## Constraints
- Discriminate on not-found ONLY -- no `is_err()` catch-alls, no substring-matching of arbitrary error strings; pin the status code via `RemoteClientError::is_not_found()` (`crates/services/src/services/remote_client.rs:91-93`, `matches!(Http { status: 404, .. })`).
- Do not change hive (`crates/remote/`) behaviour; the 404 response is correct.
- The success path (hive row exists) must remain delegation-only: local cleanup still arrives via the `task.deleted` WS event.
- Tests must drive the real seam: the served router + wiremock hive (`crates/server/tests/common/mod.rs` HiveHarness), not a unit test of a helper predicate.


## Out of scope
- The success-path hazard where a missed `task.deleted` WS event leaves the local cache stale (README 'Consider the related hazard' note) -- record, do not fix.
- Bulk reconciliation/sweep of all existing dangling `shared_task_id` rows; this fix makes each deletable on demand.
- Hive-side changes.


## Approach
Change `delete_remote_task` in `crates/server/src/routes/tasks/handlers/remote.rs` so the hive delete result is matched instead of `?`-propagated: `Ok(_)` keeps the current delegation-only path; `Err(e) if e.is_not_found()` logs a warning naming the dangling `shared_task_id` and falls through to `Task::delete(pool, task.id)` (the same local deletion the no-shared-id branch performs); any other `Err(e)` propagates as today. Cover with integration tests on `HiveHarness::configured()` mocking `DELETE /v1/tasks/{id}` as 404, 409, and 200.


## Design
One function changes: `delete_remote_task` (remote.rs:221-254).

```rust
match remote_client.delete_shared_task(shared_task_id, &request).await {
    Ok(_) => { /* existing info log; rely on task.deleted WS event */ }
    Err(e) if e.is_not_found() => {
        tracing::warn!(task_id = %task.id, shared_task_id = %shared_task_id,
            "Hive returned not-found for dangling shared_task_id; deleting locally");
        Task::delete(&deployment.db().pool, task.id).await?;
    }
    Err(e) => return Err(e.into()),
}
```

Discrimination point: `RemoteClientError::is_not_found()` -- an existing, status-pinned predicate (`Http { status: 404 }`). Auth (401/403 -> `RemoteClientError::Auth`), transport, timeout, 5xx (`Http { status: 5xx }`), and conflict (`Http { status: 409 }`) all take the final `Err` arm and abort the delete. No WS event is expected on the 404 path (the hive has no row to announce), so local deletion here cannot race the WS cleanup; `Task::delete` is idempotent against a concurrent event regardless.

Test seam: `crates/server/tests/` using `HiveHarness::configured()` -- wiremock hive + the real served router. The harness needs a small `delete()` helper (it has only get/post) and a way to seed a task with a `shared_task_id` (direct `Task::create` with `shared_task_id: Some(..)` through the harness pool).


## Decisions
- Use the existing `RemoteClientError::is_not_found()` predicate rather than any new error matching -- narrowest possible discrimination, reversible, no ADR needed.
- IRREVERSIBLE: on the 404 path, delete the local row outright (which removes the stale `shared_task_id` with it) rather than clearing the id and re-dispatching -- destroys local task data on user request; see ADR dev-docs/adr/0015-idempotent-node-delete-on-hive-not-found.md.
- Genuine-failure integration test uses HTTP 409 from the mock hive: `should_retry()` retries 5xx with exponential backoff (1s min, 3 tries), which would add ~7s to the suite; 409 exercises the same non-not-found `Err` arm without retries. The arm is status-agnostic, so coverage is equivalent.
- No ADR required: no deletion of code paths, no wire-format or contract change; hive API untouched.


## Test strategy
TS1: Integration (crates/server/tests, HiveHarness::configured): seed a task with shared_task_id, mock DELETE /v1/tasks/{id} -> 404 {"error":"shared task not found"}; assert node DELETE /api/tasks/:id returns 202 and Task::find_by_id returns None.
TS2: Integration: same seam, mock hive DELETE -> 409; assert node returns an error status (>=400) and the task row still exists.
TS3: Integration: same seam, mock hive DELETE -> 200 with a SharedTaskResponse body; assert node returns 202 and the local row is RETAINED (WS-event delegation contract preserved).

