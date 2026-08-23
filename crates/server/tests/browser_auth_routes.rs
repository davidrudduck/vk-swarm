mod common;

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
    assert_ne!(
        init.status, 401,
        "initiation must not require the session it creates"
    );
    let cb = h
        .get(
            &("/api/auth/handoff/complete?handoff_id=".to_string()
                + &uuid::Uuid::new_v4().to_string()),
        )
        .await;
    assert_ne!(cb.status, 401);
}
