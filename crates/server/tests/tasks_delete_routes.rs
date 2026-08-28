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

    let mut jar = h.authorized_jar().await;
    let resp = h
        .delete_with(&format!("/api/tasks/{task_id}"), &mut jar)
        .await;
    resp.assert_registered();
    assert_eq!(
        resp.status, 202,
        "expected 202 Accepted, body: {}",
        resp.body
    );
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

    let mut jar = h.authorized_jar().await;
    let resp = h
        .delete_with(&format!("/api/tasks/{task_id}"), &mut jar)
        .await;
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

    let mut jar = h.authorized_jar().await;
    let resp = h
        .delete_with(&format!("/api/tasks/{task_id}"), &mut jar)
        .await;
    resp.assert_registered();
    assert_eq!(
        resp.status, 202,
        "expected 202 Accepted, body: {}",
        resp.body
    );
    assert!(
        h.task_row_exists(task_id).await,
        "success path must leave local cleanup to the task.deleted WS event"
    );
}
