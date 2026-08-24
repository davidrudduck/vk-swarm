#[allow(dead_code)]
mod common;

use common::{CookieJar, HiveHarness};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

async fn login(h: &HiveHarness, subject: Uuid, app_code: &str, access_label: &str) -> CookieJar {
    let handoff_id = h
        .mock_hive_oauth(app_code, access_label, "test-refresh-token", subject)
        .await;
    let mut jar = CookieJar::fresh();
    let init = h
        .post_with(
            "/api/auth/handoff/init",
            json!({"provider":"github","return_to":"/"}),
            &mut jar,
        )
        .await;
    assert_eq!(init.status, 200, "body: {}", init.body);
    let complete = h
        .get_with(
            &format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code={app_code}"),
            &mut jar,
        )
        .await;
    assert_eq!(complete.status, 200, "body: {}", complete.body);
    jar
}

async fn seed_local_identity(h: &HiveHarness) -> (Uuid, Uuid) {
    let project_id = h
        .seed_project("continuity", &[db::models::task::TaskStatus::Todo])
        .await;
    let task_id = sqlx::query_scalar("SELECT id FROM tasks WHERE project_id = ?")
        .bind(project_id)
        .fetch_one(h.pool())
        .await
        .unwrap();
    (project_id, task_id)
}

async fn snapshot_state(h: &HiveHarness) -> (Vec<u8>, Uuid, i64) {
    let credentials = std::fs::read(h.credentials_path()).unwrap();
    let owner_bytes: Vec<u8> = sqlx::query_scalar("SELECT hive_user_id FROM node_owner")
        .fetch_one(h.pool())
        .await
        .unwrap();
    let owner = Uuid::from_slice(&owner_bytes).unwrap();
    let live_count =
        sqlx::query_scalar("SELECT COUNT(*) FROM browser_sessions WHERE revoked_at IS NULL")
            .fetch_one(h.pool())
            .await
            .unwrap();
    (credentials, owner, live_count)
}

async fn assert_local_seams(h: &HiveHarness, jar: &mut CookieJar, project_id: Uuid, task_id: Uuid) {
    assert_eq!(h.get_with("/api/info", jar).await.status, 200);
    let projects = h.get_with("/api/projects", jar).await;
    assert_eq!(projects.status, 200);
    assert!(projects.body.contains(&project_id.to_string()));
    let tasks = h
        .get_with(&format!("/api/tasks?project_id={project_id}"), jar)
        .await;
    assert_eq!(tasks.status, 200);
    assert!(tasks.body.contains(&task_id.to_string()));
    let auth_state = h.get_with("/api/auth/state", jar).await;
    assert_eq!(auth_state.status, 200);
    assert!(auth_state.body.contains("\"authorized\":true"));
    let events = h.sse_probe("/api/events", Some(jar)).await;
    assert_eq!(events.status, 200);
    assert!(
        events
            .content_type
            .as_deref()
            .is_some_and(|value| value.starts_with("text/event-stream"))
    );
}

async fn await_reached(signal: tokio::sync::oneshot::Receiver<()>) {
    tokio::time::timeout(Duration::from_secs(2), signal)
        .await
        .expect("Hive request did not reach the priority-1 responder")
        .unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn an_established_session_survives_a_planned_idle_restart() {
    let h = HiveHarness::configured().await;
    let subject = Uuid::new_v4();
    let mut jar = login(&h, subject, "code-a", "access-a").await;
    let (project_id, task_id) = seed_local_identity(&h).await;
    let snapshot = snapshot_state(&h).await;
    let old_generation = h.server_generation();
    let h = h.restart().await;
    assert_eq!(h.last_completed_server_generation(), Some(old_generation));
    assert_eq!(h.server_generation(), old_generation + 1);
    assert_local_seams(&h, &mut jar, project_id, task_id).await;
    assert_eq!(snapshot_state(&h).await, snapshot);
}

#[tokio::test]
#[serial_test::serial]
async fn restart_rejects_the_stored_hash_presented_as_a_cookie() {
    let h = HiveHarness::configured().await;
    let _jar = login(&h, Uuid::new_v4(), "code-a", "access-a").await;
    let h = h.restart().await;
    let stored_hash: String = sqlx::query_scalar("SELECT token_hash FROM browser_sessions")
        .fetch_one(h.pool())
        .await
        .unwrap();
    let mut hash_jar = CookieJar::fresh();
    hash_jar.insert("vks_browser_session", &stored_hash);
    assert_eq!(h.get_with("/api/info", &mut hash_jar).await.status, 401);
    let mut unknown_jar = CookieJar::fresh();
    unknown_jar.insert("vks_browser_session", "not-a-real-token");
    assert_eq!(h.get_with("/api/info", &mut unknown_jar).await.status, 401);
}

#[tokio::test]
#[serial_test::serial]
async fn a_revoked_session_stays_revoked_across_restart() {
    let h = HiveHarness::configured().await;
    let mut jar = login(&h, Uuid::new_v4(), "code-a", "access-a").await;
    let raw = jar.get("vks_browser_session").unwrap().to_string();
    assert_eq!(h.get_with("/api/info", &mut jar).await.status, 200);
    let logout = h
        .post_with("/api/auth/browser/logout", json!({}), &mut jar)
        .await;
    assert!(matches!(logout.status, 200 | 204));
    let h = h.restart().await;
    let mut replay = CookieJar::fresh();
    replay.insert("vks_browser_session", &raw);
    assert_eq!(h.get_with("/api/info", &mut replay).await.status, 401);
    let revoked_at: Option<i64> =
        sqlx::query_scalar("SELECT revoked_at FROM browser_sessions WHERE token_hash = ?")
            .bind(server::auth::seams::hash_token(&raw))
            .fetch_one(h.pool())
            .await
            .unwrap();
    assert!(revoked_at.is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn transport_failure_continuity() {
    let h = HiveHarness::configured().await;
    let mut jar = login(&h, Uuid::new_v4(), "code-a", "access-a").await;
    let (project_id, task_id) = seed_local_identity(&h).await;
    let snapshot = snapshot_state(&h).await;
    let baseline = h.hive_request_count("POST", "/v1/oauth/web/init").await;
    let signal = h
        .mock_hive_connection_reset("POST", "/v1/oauth/web/init")
        .await;
    let addr = h.addr();
    let handle = tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(format!("http://{addr}/api/auth/handoff/init"))
            .json(&json!({"provider":"github","return_to":"/"}))
            .send()
            .await;
    });
    await_reached(signal).await;
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/init").await,
        baseline + 1
    );
    handle.abort();
    let _ = handle.await;
    assert_local_seams(&h, &mut jar, project_id, task_id).await;
    assert_eq!(snapshot_state(&h).await, snapshot);
}

#[tokio::test]
#[serial_test::serial]
async fn timeout_in_progress_continuity() {
    let h = HiveHarness::configured().await;
    let mut jar = login(&h, Uuid::new_v4(), "code-a", "access-a").await;
    let (project_id, task_id) = seed_local_identity(&h).await;
    let snapshot = snapshot_state(&h).await;
    let baseline = h.hive_request_count("POST", "/v1/oauth/web/init").await;
    let signal = h.mock_hive_delayed("POST", "/v1/oauth/web/init").await;
    let addr = h.addr();
    let handle = tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(format!("http://{addr}/api/auth/handoff/init"))
            .json(&json!({"provider":"github","return_to":"/"}))
            .send()
            .await;
    });
    await_reached(signal).await;
    assert_local_seams(&h, &mut jar, project_id, task_id).await;
    handle.abort();
    let _ = handle.await;
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/init").await,
        baseline + 1
    );
    assert_eq!(snapshot_state(&h).await, snapshot);
}

#[tokio::test]
#[serial_test::serial]
async fn post_restart_refresh_503_continuity() {
    let h = HiveHarness::configured().await;
    let mut jar = login(&h, Uuid::new_v4(), "code-a", "access-a").await;
    let (project_id, task_id) = seed_local_identity(&h).await;
    let h = h.restart().await;
    h.write_refresh_only_credentials("post-restart-refresh")
        .await;
    let snapshot = snapshot_state(&h).await;
    let baseline = h.hive_request_count("POST", "/v1/tokens/refresh").await;
    let signal = h.mock_hive_failure("POST", "/v1/tokens/refresh", 503).await;
    let addr = h.addr();
    let cookie = jar.header_value().unwrap();
    let handle = tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .get(format!("http://{addr}/api/organizations"))
            .header("Cookie", cookie)
            .send()
            .await;
    });
    await_reached(signal).await;
    assert_eq!(
        h.hive_request_count("POST", "/v1/tokens/refresh").await,
        baseline + 1
    );
    handle.abort();
    let _ = handle.await;
    assert_local_seams(&h, &mut jar, project_id, task_id).await;
    assert_eq!(snapshot_state(&h).await, snapshot);
}

#[tokio::test]
#[serial_test::serial]
async fn hive_5xx_continuity() {
    let h = HiveHarness::configured().await;
    let mut jar = login(&h, Uuid::new_v4(), "code-a", "access-a").await;
    let (project_id, task_id) = seed_local_identity(&h).await;
    let snapshot = snapshot_state(&h).await;
    let baseline = h.hive_request_count("POST", "/v1/oauth/web/init").await;
    let signal = h.mock_hive_failure("POST", "/v1/oauth/web/init", 503).await;
    let addr = h.addr();
    let handle = tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(format!("http://{addr}/api/auth/handoff/init"))
            .json(&json!({"provider":"github","return_to":"/"}))
            .send()
            .await;
    });
    await_reached(signal).await;
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/init").await,
        baseline + 1
    );
    handle.abort();
    let _ = handle.await;
    assert_local_seams(&h, &mut jar, project_id, task_id).await;
    assert_eq!(snapshot_state(&h).await, snapshot);
}
