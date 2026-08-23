mod common;

use deployment::Deployment;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn public_surface_is_reachable_without_a_session() {
    let h = common::HiveHarness::configured().await;
    let mut jar = common::CookieJar::new();

    let health = h.get_with("/api/health", &mut jar).await;
    health.assert_registered();
    assert_eq!(health.status, 200);

    let state = h.get_with("/api/auth/state", &mut jar).await;
    state.assert_registered();
    assert_eq!(
        state.status, 200,
        "a clean browser MUST get 200, not 401: {}",
        state.body
    );
    assert!(
        state.body.contains("\"authorized\":false"),
        "body: {}",
        state.body
    );
    assert!(
        state.body.contains("oauth_available"),
        "body: {}",
        state.body
    );
    // Minimal means minimal: no config, no environment, no profile.
    for leak in ["executor", "profile", "git_repo_path", "os_type", "user_id"] {
        assert!(
            !state.body.contains(leak),
            "auth state leaked {leak}: {}",
            state.body
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn protected_api_is_denied_by_default() {
    let h = common::HiveHarness::configured().await;
    for path in [
        "/api/info",
        "/api/projects",
        "/api/tasks/all",
        "/api/auth/status",
        "/api/diagnostics",
        "/api/config",
        "/api/organizations",
    ] {
        let res = h.get(path).await;
        res.assert_registered();
        assert_eq!(
            res.status, 401,
            "{path} must be 401 without a session; body: {}",
            res.body
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn unknown_api_paths_terminate_inside_the_api_boundary() {
    let h = common::HiveHarness::configured().await;
    let res = h.get("/api/definitely-not-a-route").await;
    assert!(
        !res.is_spa_fallback(),
        "unknown /api/* fell through to SPA HTML (status {}, ct {:?})",
        res.status,
        res.content_type
    );
    assert_eq!(res.status, 404);
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_initiation_and_callback_stay_public() {
    let h = common::HiveHarness::configured().await;
    h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let init = h
        .post(
            "/api/auth/handoff/init",
            serde_json::json!({"provider": "github", "return_to": "/"}),
        )
        .await;
    init.assert_registered();
    // Handler-specific pinning: the public JSON-404 catch-all answers `{"success":false,
    // "message":"unknown api route"}` with 404 when a route is dropped, so `!= 401` alone
    // cannot prove registration (STOP trigger: status-code-alone proves routing).
    assert_eq!(init.status, 200, "body: {}", init.body);
    assert!(init.body.contains("handoff_id"), "body: {}", init.body);
    // No app_code: the registered handler answers 400 with its own message; a dropped route
    // would answer 404 JSON. No `assert_registered` here: this handler answers with HTML,
    // which `is_spa_fallback()` would misread.
    let cb = h
        .get(
            &("/api/auth/handoff/complete?handoff_id=".to_string()
                + &uuid::Uuid::new_v4().to_string()),
        )
        .await;
    assert_eq!(cb.status, 400, "body: {}", cb.body);
    assert!(cb.body.contains("Missing app_code"), "body: {}", cb.body);
}

/// Full browser login for `owner`: mounts a fresh oauth mock for `app_code`, initiates with a
/// fresh jar, completes the callback, and returns the jar now holding the session cookie.
async fn login(h: &common::HiveHarness, owner: uuid::Uuid, app_code: &str) -> common::CookieJar {
    let handoff_id = h
        .mock_hive_oauth(
            app_code,
            &format!("label-{app_code}"),
            "test-refresh-token",
            owner,
        )
        .await;
    let mut jar = common::CookieJar::fresh();
    let res = h
        .post_with(
            "/api/auth/handoff/init",
            json!({"provider": "github", "return_to": "/"}),
            &mut jar,
        )
        .await;
    assert_eq!(res.status, 200, "body: {}", res.body);
    let done = h
        .get_with(
            &format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code={app_code}"),
            &mut jar,
        )
        .await;
    assert_eq!(done.status, 200, "body: {}", done.body);
    jar
}

/// The pinned owner's hive subject from `node_owner`.
async fn stored_owner_uuid(pool: &sqlx::SqlitePool) -> uuid::Uuid {
    let pinned: Vec<u8> = sqlx::query_scalar("SELECT hive_user_id FROM node_owner")
        .fetch_one(pool)
        .await
        .unwrap();
    uuid::Uuid::from_slice(&pinned).unwrap()
}

/// Count of browser sessions that are still live (not revoked).
async fn live_session_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM browser_sessions WHERE revoked_at IS NULL")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// `spawn_remote_sync` installs the real handle asynchronously, so wait boundedly until the
/// slot is populated. Public `Deployment::spawn_remote_sync(ShareConfig)` only — never a
/// private RemoteSync spawn, fake handle, or direct slot mutation.
async fn wait_until_sync_slot_is_some(d: &server::DeploymentImpl) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2_000);
    while d.share_sync_handle().lock().await.is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "remote sync handle was never installed"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[tokio::test]
#[serial_test::serial]
async fn browser_logout_revokes_the_presented_raw_token_only_and_keeps_real_sync() {
    let h = common::HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let mut a = login(&h, owner, "code-a").await;
    let mut b = login(&h, owner, "code-b").await;
    let raw_a = a
        .get("vks_browser_session")
        .expect("session cookie")
        .to_owned();
    let hash_a = server::auth::seams::hash_token(&raw_a);
    assert_eq!(h.get_with("/api/info", &mut a).await.status, 200);

    let share = services::services::share::ShareConfig::from_env().expect("harness share config");
    h.deployment().spawn_remote_sync(share);
    wait_until_sync_slot_is_some(h.deployment()).await;

    let out = h
        .post_with("/api/auth/browser/logout", json!({}), &mut a)
        .await;
    assert!(matches!(out.status, 200 | 204));
    assert!(
        out.set_cookie
            .iter()
            .any(|v| v.starts_with("vks_browser_session=") && v.contains("Max-Age=0"))
    );

    let mut replay = common::CookieJar::fresh();
    replay.insert("vks_browser_session", &raw_a);
    assert_eq!(
        h.get_with("/api/info", &mut replay).await.status,
        401,
        "replay the captured raw token, not the now-empty presenting jar"
    );
    let revoked_at: Option<i64> =
        sqlx::query_scalar("SELECT revoked_at FROM browser_sessions WHERE token_hash = ?")
            .bind(hash_a)
            .fetch_one(h.pool())
            .await
            .unwrap();
    assert!(revoked_at.is_some());
    assert_eq!(h.get_with("/api/info", &mut b).await.status, 200);
    assert!(
        h.deployment().share_sync_handle().lock().await.is_some(),
        "browser logout must leave the real sync handle running"
    );
    assert!(
        h.deployment().node_cache_sync_is_running().await,
        "browser logout must leave node-cache synchronization running"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn hive_disconnect_revokes_all_sessions_stops_real_sync_and_keeps_owner() {
    use services::services::share::ShareConfig;

    let h = common::HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let mut a = login(&h, owner, "code-a").await;
    let mut b = login(&h, owner, "code-b").await;
    let share = ShareConfig::from_env().expect("harness share config");
    h.deployment().spawn_remote_sync(share);
    wait_until_sync_slot_is_some(h.deployment()).await;

    let res = h.post_with("/api/auth/logout", json!({}), &mut a).await;
    assert!(matches!(res.status, 200 | 204));
    assert!(h.deployment().share_sync_handle().lock().await.is_none());
    assert!(
        !h.deployment().node_cache_sync_is_running().await,
        "explicit Hive disconnect must stop every Hive synchronization task"
    );
    assert_eq!(h.get_with("/api/info", &mut a).await.status, 401);
    assert_eq!(h.get_with("/api/info", &mut b).await.status, 401);
    assert!(!h.credentials_path().exists());
    assert_eq!(stored_owner_uuid(h.pool()).await, owner);
    assert_eq!(live_session_count(h.pool()).await, 0);
}

/// SC8/D4: the pinned owner survives disconnect, so a DIFFERENT Hive subject attempting a full
/// login afterwards is rejected with 400 and does not replace the retained owner.
#[tokio::test]
#[serial_test::serial]
async fn a_different_subject_after_disconnect_is_rejected_and_the_owner_retained() {
    let h = common::HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let mut a = login(&h, owner, "code-a").await;
    let out = h.post_with("/api/auth/logout", json!({}), &mut a).await;
    assert!(matches!(out.status, 200 | 204), "status: {}", out.status);

    let intruder = Uuid::new_v4();
    let id = h
        .mock_hive_oauth(
            "code-intruder",
            "label-intruder",
            "intruder-refresh",
            intruder,
        )
        .await;
    let mut c = common::CookieJar::fresh();
    let init = h
        .post_with(
            "/api/auth/handoff/init",
            json!({"provider": "github", "return_to": "/"}),
            &mut c,
        )
        .await;
    assert_eq!(init.status, 200, "body: {}", init.body);
    let res = h
        .get_with(
            &format!("/api/auth/handoff/complete?handoff_id={id}&app_code=code-intruder"),
            &mut c,
        )
        .await;
    assert_eq!(res.status, 400, "body: {}", res.body);
    assert!(
        res.body.contains("owned by a different account"),
        "a different subject is the owner mismatch, not a generic failure: {}",
        res.body
    );
    assert_eq!(
        stored_owner_uuid(h.pool()).await,
        owner,
        "a rejected subject must not replace the retained owner"
    );
}

/// The browser-scoped logout is a PROTECTED route: without a session cookie the boundary layer
/// answers 401 for both GET and POST, before the handler ever runs.
#[tokio::test]
#[serial_test::serial]
async fn anonymous_browser_logout_is_rejected_before_the_handler() {
    let h = common::HiveHarness::configured().await;
    let mut anon = common::CookieJar::fresh();
    let post = h
        .post_with("/api/auth/browser/logout", json!({}), &mut anon)
        .await;
    post.assert_registered();
    assert_eq!(post.status, 401, "body: {}", post.body);
    let get = h.get_with("/api/auth/browser/logout", &mut anon).await;
    get.assert_registered();
    assert_eq!(get.status, 401, "body: {}", get.body);
}

/// Integrated race 1 (the incident symptom): a disconnect that returns while browser A's
/// callback is mid-flight — its profile response deliberately delayed — must fence A's commit:
/// generic 400, no session row for A, every prior live session revoked, no credentials, sync
/// slot empty, and the pinned owner retained.
#[tokio::test]
#[serial_test::serial]
async fn disconnect_during_an_in_flight_callback_leaves_no_session_credentials_or_sync() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let h = common::HiveHarness::configured().await;
        let owner = Uuid::new_v4();
        let mut b = login(&h, owner, "code-b").await;

        let a_id = h
            .mock_hive_oauth("code-a", "delayed-a", "test-refresh-token", owner)
            .await;
        let profile_arrived = h
            .delay_hive_profile("delayed-a", owner, Duration::from_millis(400))
            .await;

        let mut a = common::CookieJar::fresh();
        let init = h
            .post_with(
                "/api/auth/handoff/init",
                json!({"provider": "github", "return_to": "/"}),
                &mut a,
            )
            .await;
        assert_eq!(init.status, 200, "body: {}", init.body);

        let addr = h.addr();
        let cookie = a
            .header_value()
            .expect("browser A must hold a binding cookie");
        let cb = format!("/api/auth/handoff/complete?handoff_id={a_id}&app_code=code-a");
        let request = tokio::spawn(async move {
            let res = reqwest::Client::new()
                .get(format!("http://{addr}{cb}"))
                .header(reqwest::header::COOKIE, cookie)
                .send()
                .await
                .expect("callback request must be issued");
            let status = res.status().as_u16();
            let body = res.text().await.unwrap();
            (status, body)
        });

        tokio::time::timeout(Duration::from_secs(2), profile_arrived)
            .await
            .expect("profile never reached the delayed Hive mock")
            .expect("delayed-profile signal channel closed");

        // The valid profile response is still ~400ms away: B disconnects NOW.
        let out = h.post_with("/api/auth/logout", json!({}), &mut b).await;
        assert!(
            matches!(out.status, 200 | 204),
            "disconnect failed: {} {}",
            out.status,
            out.body
        );

        let (status, body) = request.await.expect("callback task panicked");
        assert_eq!(status, 400, "body: {body}");
        assert!(
            body.contains("Sign-in could not be completed"),
            "must be the generic browser-login failure message: {body}"
        );
        assert!(
            !body.contains("owned by a different account"),
            "an interrupted callback is a disconnect, never an owner mismatch: {body}"
        );

        // No session row for A exists: exactly B's one revoked row remains.
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM browser_sessions")
            .fetch_one(h.pool())
            .await
            .unwrap();
        assert_eq!(total, 1, "browser A must not have minted a session");
        assert_eq!(
            live_session_count(h.pool()).await,
            0,
            "all previously live sessions must be revoked"
        );
        assert!(!h.credentials_path().exists());
        assert!(h.deployment().share_sync_handle().lock().await.is_none());
        assert_eq!(
            stored_owner_uuid(h.pool()).await,
            owner,
            "disconnect retains the pinned owner"
        );
    })
    .await
    .expect("disconnect during in-flight callback timed out (lock regression?)");
}

/// Integrated race 2: a handoff initiated BEFORE disconnect is durably invalidated in the DB —
/// its late callback 400s even though it claims after the epoch bump, its row is terminal, no
/// session is live, and credentials are absent.
#[tokio::test]
#[serial_test::serial]
async fn a_pending_callback_from_before_disconnect_is_durably_invalidated() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let h = common::HiveHarness::configured().await;
        let owner = Uuid::new_v4();
        // Initiate A but do NOT call its callback yet.
        let a_id = h
            .mock_hive_oauth("code-a", "label-a", "test-refresh-token", owner)
            .await;
        let mut a = common::CookieJar::fresh();
        let init = h
            .post_with(
                "/api/auth/handoff/init",
                json!({"provider": "github", "return_to": "/"}),
                &mut a,
            )
            .await;
        assert_eq!(init.status, 200, "body: {}", init.body);

        let mut b = login(&h, owner, "code-b").await;
        let out = h.post_with("/api/auth/logout", json!({}), &mut b).await;
        assert!(matches!(out.status, 200 | 204), "status: {}", out.status);

        // A's pre-disconnect callback is durably dead.
        let res = h
            .get_with(
                &format!("/api/auth/handoff/complete?handoff_id={a_id}&app_code=code-a"),
                &mut a,
            )
            .await;
        assert_eq!(res.status, 400, "body: {}", res.body);

        assert_eq!(live_session_count(h.pool()).await, 0);
        assert!(!h.credentials_path().exists());
        let state: String =
            sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
                .bind(a_id)
                .fetch_one(h.pool())
                .await
                .unwrap();
        assert_eq!(state, "claimed", "the handoff row must be terminal");
    })
    .await
    .expect("pending-callback invalidation timed out (lock regression?)");
}

/// Integrated race 3: disconnect must not permanently disable the node — a wholly new login
/// for the SAME pinned owner succeeds afterwards, reinstalls RemoteSync, restarts node-cache
/// sync, and leaves exactly one live session with credentials present.
#[tokio::test]
#[serial_test::serial]
async fn a_fresh_login_after_disconnect_still_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let h = common::HiveHarness::configured().await;
        let owner = Uuid::new_v4();
        let mut b = login(&h, owner, "code-b").await;
        let out = h.post_with("/api/auth/logout", json!({}), &mut b).await;
        assert!(matches!(out.status, 200 | 204), "status: {}", out.status);

        let mut c = login(&h, owner, "code-c").await;
        assert_eq!(h.get_with("/api/info", &mut c).await.status, 200);
        assert_eq!(live_session_count(h.pool()).await, 1);
        assert!(h.credentials_path().exists());
        assert!(h.deployment().share_sync_handle().lock().await.is_some());
        assert!(h.deployment().node_cache_sync_is_running().await);
    })
    .await
    .expect("fresh login after disconnect timed out (lock regression?)");
}

/// Integrated race 4: an in-flight token refresh holding `refresh_guard` must neither deadlock
/// disconnect nor undo it — the disconnect request finishes after the refresh guard becomes
/// available and the final credentials path is absent.
#[tokio::test]
#[serial_test::serial]
async fn disconnect_is_not_undone_by_an_in_flight_token_refresh() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let h = common::HiveHarness::configured().await;
        let owner = Uuid::new_v4();
        let mut b = login(&h, owner, "code-b").await;
        h.write_refresh_only_credentials("test-refresh-token").await;
        let arrived = h
            .mock_hive_delayed_json(
                "POST",
                "/v1/tokens/refresh",
                400,
                // INVALID access token on purpose (plan amendment 636cba61): the refresher
                // errors with RemoteClientError::Token WITHOUT saving anything, so a handler
                // that wrongly takes refresh_guard before client.logout() re-enters the
                // non-reentrant guard on the same task and deadlocks into the 30s timeout —
                // the mutation discrimination a valid-token body cannot provide.
                serde_json::json!({
                    "access_token": "not-a-jwt",
                    "refresh_token": "test-refresh-token"
                }),
            )
            .await;
        let client = h
            .deployment()
            .remote_client()
            .expect("configured harness must have a remote client");
        let refresher = tokio::spawn(async move { client.access_token().await });
        tokio::time::timeout(Duration::from_secs(2), arrived)
            .await
            .expect("refresh never reached the delayed Hive mock")
            .expect("delayed-responder signal channel closed");

        let out = h.post_with("/api/auth/logout", json!({}), &mut b).await;
        assert!(
            matches!(out.status, 200 | 204),
            "disconnect failed: {} {}",
            out.status,
            out.body
        );
        assert!(
            !h.credentials_path().exists(),
            "the in-flight refresh must not out-live the disconnect's credential clear"
        );
        let _ = refresher.await;
    })
    .await
    .expect("disconnect during in-flight token refresh timed out (lock regression?)");
}
