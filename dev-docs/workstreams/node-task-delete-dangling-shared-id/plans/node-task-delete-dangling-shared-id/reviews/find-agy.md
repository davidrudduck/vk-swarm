I will start by listing the directory structure and reading the specified files to understand the plan and verify anchors against the repository.
I will list the contents of the `phase-1` subdirectory to see all task breakdown files.
I will read the spec file `docs/superpowers/specs/2026-08-05-node-task-delete-dangling-shared-id.md` to understand the goal and requirements.
I will read the overall plan file `docs/plans/node-task-delete-dangling-shared-id/plan.md`.
I will read the phase file `docs/plans/node-task-delete-dangling-shared-id/phase-1-idempotent-delete.md`.
I will read task file `docs/plans/node-task-delete-dangling-shared-id/phase-1/001-extend-hiveharness-with-delete-seed-shared-task-task-row-exists.md`.
I will read task file `docs/plans/node-task-delete-dangling-shared-id/phase-1/002-tdd-dangling-shared-task-id-delete-falls-through-locally-non-not-found-still-aborts.md`.
I will read `crates/server/src/routes/tasks/handlers/remote.rs` to verify the anchor and how `delete_remote_task` is structured.
I will read `crates/server/src/routes/tasks/handlers/core.rs` to see how `delete_task` is implemented and how it delegates to `delete_remote_task`.
I will read `crates/services/src/services/remote_client.rs` to understand `RemoteClientError`, `delete_shared_task`, and check for `is_not_found()`.
I will search for the definition of `SharedTaskResponse` in the codebase to verify its shape.
I will view the `SharedTaskResponse` struct in `crates/remote/src/routes/tasks.rs` to inspect its fields and their types.
I will search for the definitions of `SharedTask` and `UserData` to confirm their fields.
I will view the `SharedTask` struct definition in `crates/remote/src/db/tasks.rs` to see its exact fields and types.
I will search for the definition of `TaskStatus` in the `remote` or other crates to check its deserialization behavior.
I will view the `TaskStatus` enum definition in `crates/remote/src/db/tasks.rs`.
I will search for the definition of `UserData` to verify its fields.
I will view the `UserData` struct definition in `crates/remote/src/db/users.rs`.
I will read `crates/server/tests/common/mod.rs` to inspect `HiveHarness`, how the `Deployment` is configured, how remote client is mocked, and how request routing/auth are handled.
I will read the rest of `crates/server/tests/common/mod.rs`.
I will search for the definition of the `CreateTask` struct to verify its fields.
I will view `crates/db/src/models/task/mod.rs` around line 104 to check the fields of `CreateTask`.
I will search for `find_by_id` in the `db` crate to see how it is defined on the `Task` model.
I will search for the definition of `Task::create` to verify its arguments.
I will view the `Task::create` function signature in `crates/db/src/models/task/queries.rs`.
I will search for `Task::delete` in the `db` crate to see how it is defined.
I will view the `Task::delete` signature in `crates/db/src/models/task/queries.rs`.
I will search for the definition of the `db` method in the `deployment` or related crates to verify the type returned.
I will view `crates/deployment/src/lib.rs` to see what type `db()` returns and what `DBService` provides.
I will search for `DBService` to check its definition and verify that it has a `pool` field.
I will search for `struct DBService` across the whole repository.
I will view `crates/db/src/lib.rs` around line 307 to confirm that it has a public `pool` field.
I will search for references to `delete_task` in the router files to see how the DELETE route is set up.
I will view `crates/server/src/routes/tasks/mod.rs` to see the full route construction and middleware for the delete endpoint.
I will view the rest of `crates/server/src/routes/tasks/mod.rs` to see the rest of the router setup.
I will search for `tasks::router` or `.nest` in the `routes` directory to find how the tasks endpoints are exposed to the client.
I will view `crates/server/src/routes/mod.rs` around line 55 to see the nested routing.
I will view the rest of `crates/server/src/routes/mod.rs` to see where `/api` is prefixed and how the fallback router is set up.
I will search for `.nest` or `/tasks` in `crates/server/src/routes/projects` to see if there is another route nesting tasks under projects.
I will search for `load_task_middleware` to understand how task loading works and what requirements/dependencies it has.
I will view `crates/server/src/middleware/model_loaders.rs` around line 348 to see if there is any parameter (like project_id) or if it extracts task_id directly from the path.
I will view more of `load_task_middleware` to check the fallback/lookup path when the task isn't found locally.
I will view the rest of `load_task_middleware` to see the behavior when the task is not found locally and the request is not a GET (like a DELETE).
I will view lines 451 to 480 of `load_task_middleware`.
I will view lines 481 to 495 of `load_task_middleware` to see the final fallback behavior.
I will view lines 496 to 510 of `load_task_middleware` to see the final fallback when task load fails.
I will view lines 511 to 525 of `load_task_middleware` to see how it rejects non-GET requests if the task is missing.
I will view `crates/server/tests/nodes_routes.rs` to see if there is any sibling pattern or imports that should be followed.
I will view `crates/server/Cargo.toml` to verify the dependencies.
I will view the rest of `crates/server/Cargo.toml` to verify test dependencies like `serial_test`.
I will view `crates/server/src/error.rs` to verify that `ApiError` can be constructed from `sqlx::Error`.
I will search for `parent_task_id` in migrations to see what the foreign key action is when the parent task is deleted.
I will search for `delete_by_shared_task_id` or `delete_stale_shared_tasks` to see how the sync system handles remote task deletion.
I will search for `delete_by_shared_task_id` in the `services/src` directory.
I will search for `delete_by_shared_task_id` across the repository to locate its uses.
I will list the contents of `crates/services/src/services` to find the sync or runner service files.
I will search for `Task::delete` calls in `crates/services/src` to see how the sync service deletes a task.
I will grep for `delete` in `crates/db/src/models/task/sync.rs` to see what functions perform deletion.
I will search for `preupdate` to see if the codebase uses SQLite preupdate hooks to listen to database-level changes (like Electric SQL deletes) and perform cleanup.
I will view `crates/services/src/services/events.rs` to see what happens when a task deletion is caught in the SQLite preupdate hook.
I will search for `batch_cleanup_worktrees` or similar worktree cleanup triggers in the `services` crate to see how and where worktree deletion is normally handled.
I will search for all references to `batch_cleanup_worktrees` in the codebase.
I will search for `"/api/tasks"` in the tests directory to see what URLs other integration tests use to interact with tasks.
I will list the files in the tests directory to see what test files exist.
I will view `crates/server/tests/projects_with_stats.rs` to see what APIs it tests.
I will search for `"/api"` in `mcp_context_test.rs` to see if there are task-related calls.
I will view `crates/server/src/routes/projects/mod.rs` to verify project nesting.
I will search for `credentials.json` in the `services` crate to verify how the credentials path is determined.
I will search for `credentials.json` in the whole repository.
I will search for `reqwest` in `crates/server/tests/common/mod.rs` to verify if it is imported.
I will search for all references to `Task::delete` to verify what arguments are passed.
I will view the `IntoResponse` implementation for `ApiError` in `crates/server/src/error.rs`.
I will read the decisions ledger file `docs/plans/node-task-delete-dangling-shared-id/decisions-ledger.md` to verify it matches constraints.
I will search for `remote_client` signature in `crates/deployment` to verify what error type it returns.
I will search for `remote_client` definition or calls in the codebase.
I will search for `remote_client` in `crates/server` and `crates/services` to find where it is defined.
I will search for the definition of the `remote_client` function across the whole repository.
I will view `crates/server/src/error.rs` around lines 100 to 150 to see if `RemoteClientNotConfigured` has a `From` implementation to `ApiError`.
I will search for `to_rfc3339` in the tests directory to check if it's already used for mock dates.
I will search for `to_rfc3339` across the entire codebase to see where it is used.
I will search for `delete_shared_task` in the codebase to make sure there are no other places where deleting a task could fail.
I will list files in `docs/plans/node-task-delete-dangling-shared-id/reviews/` to see what reviews currently exist.
