# node-task-delete-dangling-shared-id Plan

## Spec
docs/superpowers/specs/2026-08-05-node-task-delete-dangling-shared-id.md

## Approach
Two surgical tasks in one phase. Task 001 extends the existing integration harness (crates/server/tests/common/mod.rs HiveHarness) with the three helpers the new tests need: an HTTP `delete()` driver (mirroring the existing get/post), a `seed_shared_task()` seeder that inserts a task with `shared_task_id: Some(..)` through the deployment pool, and a `task_row_exists()` assertion helper. Task 002 is the TDD task: it first writes crates/server/tests/tasks_delete_routes.rs with three serial tests driving the REAL served router against a wiremock hive (404 dangling -> 202 + row gone; 409 conflict -> error + row survives; 200 success -> 202 + row retained for WS cleanup), then makes them green by replacing the `?`-propagation in `delete_remote_task` (crates/server/src/routes/tasks/handlers/remote.rs) with a three-arm match: Ok keeps the delegation-only path, `Err(e) if e.is_not_found()` warns and falls through to `Task::delete`, any other Err propagates unchanged.

Discrimination is pinned to the existing status-pinned predicate `RemoteClientError::is_not_found()` (Http { status: 404 }) -- never `is_err()`, never string matching. The genuine-failure test uses 409 rather than 5xx because `should_retry()` retries 5xx with exponential backoff and would slow the suite ~7s; the non-not-found Err arm is status-agnostic so coverage is equivalent (recorded in the spec Decisions).

Irreversible surface (local row deletion on user-requested delete) is covered by ADR-0015; the tasks themselves are ordinary edits/creates, no human gate needed.


## Phases
- **Phase 1: idempotent-delete** — A dangling shared_task_id no longer makes a node task undeletable: hive 404 falls through to local deletion, genuine hive failures still abort, success path unchanged.

## Tasks

| id | phase | title | deps | conflicts |
|---|---:|---|---|---|
| 001 | 1 | Extend HiveHarness with delete(), seed_shared_task(), task_row_exists() | dep: none | conflicts: none |
| 002 | 1 | TDD: dangling shared_task_id delete falls through locally; non-not-found still aborts | dep: 001 | conflicts: none |
