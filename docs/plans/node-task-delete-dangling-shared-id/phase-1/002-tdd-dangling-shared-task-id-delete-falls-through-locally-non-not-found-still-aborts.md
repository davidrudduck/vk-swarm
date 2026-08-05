---
id: "002"
phase: 1
title: "TDD: dangling shared_task_id delete falls through locally; non-not-found still aborts"
status: passed
depends_on: ["001"]
parallel: false
conflicts_with: []
files:
  - "crates/server/tests/tasks_delete_routes.rs"
  - "crates/server/src/routes/tasks/handlers/remote.rs"
  - "crates/server/Cargo.toml"
siblings: ["crates/server/tests/nodes_routes.rs"]
irreversible: false
scope_test: "crates/server/tests/tasks_delete_routes.rs"
allowed_change: mixed
covers_criteria: ["SC1","SC2","SC3"]
covers_tests: ["TS1","TS2","TS3"]
---
## Failing test (write first)
Create crates/server/tests/tasks_delete_routes.rs with EXACTLY:

```rust
//! Integration tests for DELETE /api/tasks/{task_id} against a mocked hive.
//!
//! Pins F-2026-08-05-01: a dangling `shared_task_id` (hive 404) must fall through to
//! local deletion, while genuine hive failures must still abort the delete.

mod common;

use common::HiveHarness;
use uuid::Uuid;

/// TS1/SC1/SC3: hive 404 ("shared task not found") -> local deletion proceeds, and the
/// fall-through emits a tracing warn naming the dangling shared_task_id (SC3).
#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn delete_task_with_dangling_shared_id_deletes_locally() {
    let h = HiveHarness::configured().await;
    let project_id = h.seed_project("dangling-delete", &[]).await;
    let shared_id = Uuid::new_v4();
    let task_id = h.seed_shared_task(project_id, shared_id).await;

    h.mock_json(
        "DELETE",
        &format!("/v1/tasks/{shared_id}"),
        404,
        serde_json::json!({"error": "shared task not found"}),
    )
    .await;

    let resp = h.delete(&format!("/api/tasks/{task_id}")).await;
    resp.assert_registered();
    assert_eq!(resp.status, 202, "expected 202 Accepted, body: {}", resp.body);
    assert!(
        !h.task_row_exists(task_id).await,
        "task row must be deleted locally when the hive reports not-found"
    );
    // SC3: the fall-through warn names the dangling shared_task_id.
    assert!(
        logs_contain("dangling shared_task_id"),
        "expected the not-found fall-through to log a warn mentioning 'dangling shared_task_id'"
    );
    assert!(
        logs_contain(&shared_id.to_string()),
        "expected the warn to name the concrete dangling shared_task_id"
    );
}

/// TS2/SC2: a non-not-found hive failure must abort the delete.
/// 409 (not 5xx) so `RemoteClientError::should_retry()` adds no backoff delay;
/// the non-not-found `Err` arm is status-agnostic (spec Decisions).
#[tokio::test]
#[serial_test::serial]
async fn delete_task_aborts_when_hive_fails_with_conflict() {
    let h = HiveHarness::configured().await;
    let project_id = h.seed_project("conflict-delete", &[]).await;
    let shared_id = Uuid::new_v4();
    let task_id = h.seed_shared_task(project_id, shared_id).await;

    h.mock_json(
        "DELETE",
        &format!("/v1/tasks/{shared_id}"),
        409,
        serde_json::json!({"error": "version conflict"}),
    )
    .await;

    let resp = h.delete(&format!("/api/tasks/{task_id}")).await;
    resp.assert_registered();
    assert!(
        resp.status >= 400,
        "expected an error status, got {} body {}",
        resp.status,
        resp.body
    );
    assert!(
        h.task_row_exists(task_id).await,
        "task row must survive a non-not-found hive failure"
    );
}

/// TS3: the success path stays delegation-only (local cleanup via the task.deleted WS event).
#[tokio::test]
#[serial_test::serial]
async fn delete_task_success_path_retains_local_row_for_ws_cleanup() {
    let h = HiveHarness::configured().await;
    let project_id = h.seed_project("success-delete", &[]).await;
    let shared_id = Uuid::new_v4();
    let task_id = h.seed_shared_task(project_id, shared_id).await;

    let now = chrono::Utc::now().to_rfc3339();
    h.mock_json(
        "DELETE",
        &format!("/v1/tasks/{shared_id}"),
        200,
        serde_json::json!({
            "task": {
                "id": shared_id,
                "organization_id": Uuid::new_v4(),
                "title": "success-delete-task",
                "status": "todo",
                "version": 2,
                "deleted_at": now,
                "created_at": now,
                "updated_at": now
            },
            "user": null
        }),
    )
    .await;

    let resp = h.delete(&format!("/api/tasks/{task_id}")).await;
    resp.assert_registered();
    assert_eq!(resp.status, 202, "expected 202 Accepted, body: {}", resp.body);
    assert!(
        h.task_row_exists(task_id).await,
        "success path must leave local cleanup to the task.deleted WS event"
    );
}
```

RUN RED FIRST: `cargo test -p server --test tasks_delete_routes` — `delete_task_with_dangling_shared_id_deletes_locally` MUST FAIL (node currently propagates the hive 404, so resp.status is 404, not 202). The 409 and 200 tests pass already (they pin existing behaviour against regression). Record the red output in the decisions ledger, then apply the Change below.


## Change
**File:** crates/server/Cargo.toml
**Anchor:** `[dev-dependencies]` block (~L67-73), the line `jsonwebtoken = { version = "10.2.0", features = ["rust_crypto"] }`.
**Before:**

```toml
jsonwebtoken = { version = "10.2.0", features = ["rust_crypto"] }
```

**After:**

```toml
jsonwebtoken = { version = "10.2.0", features = ["rust_crypto"] }
tracing-test = { version = "0.2", features = ["no-env-filter"] }
```

(`tracing-test 0.2` is already in the workspace lockfile via crates/services; `no-env-filter` is required so the subscriber captures events whose target is the `server` crate, not the integration-test crate.)

**File:** crates/server/src/routes/tasks/handlers/remote.rs
**Anchor:** function `delete_remote_task`, the `if let Some(shared_task_id) = task.shared_task_id {` block (~L226-241).
**Before:**

```rust
    if let Some(shared_task_id) = task.shared_task_id {
        let remote_client = deployment.remote_client()?;
        let request = DeleteSharedTaskRequest { version: None };
        remote_client
            .delete_shared_task(shared_task_id, &request)
            .await?;

        tracing::info!(
            task_id = %task.id,
            shared_task_id = %shared_task_id,
            "Deleted remote task via Hive; local cache will be cleaned by WebSocket sync"
        );
        // NOTE: Do NOT delete locally here - WebSocket handler will process
        // the "task.deleted" event and clean up the local cache in a transaction
    } else {
```

**After:**

```rust
    if let Some(shared_task_id) = task.shared_task_id {
        let remote_client = deployment.remote_client()?;
        let request = DeleteSharedTaskRequest { version: None };
        match remote_client
            .delete_shared_task(shared_task_id, &request)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    task_id = %task.id,
                    shared_task_id = %shared_task_id,
                    "Deleted remote task via Hive; local cache will be cleaned by WebSocket sync"
                );
                // NOTE: Do NOT delete locally here - WebSocket handler will process
                // the "task.deleted" event and clean up the local cache in a transaction
            }
            // Idempotent delete: the hive row is already gone (dangling shared_task_id,
            // e.g. after a hive cutover truncation), so the desired end state is reached
            // remotely — fall through to the same local deletion the no-shared-id branch
            // performs. Discriminates on 404 ONLY via RemoteClientError::is_not_found();
            // auth/transport/timeout/5xx/conflict take the arm below and still abort.
            // (F-2026-08-05-01, ADR-0015)
            Err(e) if e.is_not_found() => {
                tracing::warn!(
                    task_id = %task.id,
                    shared_task_id = %shared_task_id,
                    "Hive returned not-found for dangling shared_task_id; deleting task locally"
                );
                Task::delete(&deployment.db().pool, task.id).await?;
            }
            Err(e) => return Err(e.into()),
        }
    } else {
```

`Task` is already imported in this file; `is_not_found()` exists on `RemoteClientError` (crates/services/src/services/remote_client.rs:91-93). Do NOT add imports.


## Allowed moves
ONLY: (a) create crates/server/tests/tasks_delete_routes.rs with the exact content in Failing test; (b) add the single tracing-test dev-dependency line to crates/server/Cargo.toml; (c) replace the exact Before block with the exact After block in delete_remote_task. Do NOT touch the `else` branch, the 202 return, `delete_task` in core.rs, the hive (crates/remote/), or RemoteClientError. NEVER widen the `Err(e) if e.is_not_found()` guard to `Err(_)`/`is_err()` or string matching.


## STOP triggers
- the Before block does not match remote.rs verbatim at the stated anchor
- `RemoteClientError::is_not_found()` is absent or its arm fails to compile
- the red run fails on a test OTHER than delete_task_with_dangling_shared_id_deletes_locally (e.g. the 200-success mock body fails to deserialize -> fix the mock body ONLY after confirming SharedTask's serde shape, never by changing production code)
- making the tests green would require editing any file beyond the two listed


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh node-task-delete-dangling-shared-id 002` exits 0
