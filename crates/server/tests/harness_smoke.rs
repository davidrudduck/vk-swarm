mod common;

#[tokio::test]
#[serial_test::serial]
async fn harness_serves_a_configured_hive() {
    let h = common::HiveHarness::configured().await;
    h.mock_json(
        "GET",
        "/v1/organizations",
        200,
        serde_json::json!({"organizations": []}),
    )
    .await;
    let res = h.get("/api/organizations").await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);
    assert!(res.body.contains("\"success\":true"), "body: {}", res.body);
}

// A 401 here means the credential seeding in `configured()` did not take effect — see
// Amendment B. Do NOT "fix" it by relaxing this assertion to `assert_ne!(res.status, 404)`;
// the 200 is the frozen spec's required SC1/SC4 signal and is the whole reason task 099 exists.

#[tokio::test]
#[serial_test::serial]
async fn harness_serves_an_absent_hive() {
    let h = common::HiveHarness::hive_absent().await;
    let mut jar = h.authorized_jar().await;
    let res = h.get_with("/api/organizations", &mut jar).await;
    res.assert_registered();
    assert_eq!(
        res.status, 503,
        "absent hive must be the specific HiveNotConfigured 503 — list_organizations goes through \
         deployment.remote_client()? (organizations.rs:72), same as the four swarm routes; body: {}",
        res.body
    );
}

#[tokio::test]
#[serial_test::serial]
async fn harness_detects_an_unregistered_route() {
    let h = common::HiveHarness::hive_absent().await;

    // A path that is registered today.
    let ok = h.get("/api/health").await;
    assert!(
        !ok.is_spa_fallback(),
        "/api/health must be a real route, got {:?}",
        ok.content_type
    );

    // A path that is NOT registered. It returns 200 + SPA HTML, NOT 404 — which is
    // exactly why assert_ne!(404) cannot prove registration in this codebase.
    let missing = h.get("/api/definitely-not-a-route").await;
    assert!(
        missing.is_spa_fallback(),
        "expected the SPA fallback for an unregistered route, got status {} body {:.80}",
        missing.status,
        missing.body
    );
}

#[tokio::test]
#[serial_test::serial]
async fn jars_are_independent_and_capture_set_cookie() {
    let h = common::HiveHarness::configured().await;
    let mut a = common::CookieJar::new();
    let b = common::CookieJar::new();
    a.insert("vks_probe", "A");
    assert_eq!(a.header_value().as_deref(), Some("vks_probe=A"));
    assert_eq!(b.header_value(), None, "jars must not share state");

    let res = h.get_with("/api/health", &mut a).await;
    res.assert_registered();
    assert_eq!(res.status, 200);
    assert!(res.set_cookie.is_empty(), "health sets no cookie: {:?}", res.set_cookie);
}

#[tokio::test]
#[serial_test::serial]
async fn hive_oauth_mocks_hand_out_successive_handoff_ids() {
    let h = common::HiveHarness::configured().await;
    let sub = uuid::Uuid::new_v4();
    let first = h.mock_hive_oauth("code-1", "acc-1", "ref-1", sub).await;
    let second = h.mock_hive_oauth("code-2", "acc-2", "ref-2", sub).await;
    assert_ne!(first, second);
    for (m, p) in [("POST", "/v1/oauth/web/init"), ("POST", "/v1/oauth/web/redeem"),
                   ("GET", "/v1/profile")] {
        assert!(h.hive_mock_registered(m, p).await, "missing mock for {m} {p}");
    }
    // Two successive initiations must receive the two DIFFERENT ids, in order. Without
    // `.up_to_n_times(1)` on the init mock, wiremock's first-match-wins resolution would return
    // `first` twice and every two-login test would fail for an unrelated reason.
    let mut jar = common::CookieJar::new();
    let r1 = h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut jar).await;
    let r2 = h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut jar).await;
    assert!(r1.body.contains(&first.to_string()), "body: {}", r1.body);
    assert!(r2.body.contains(&second.to_string()), "body: {}", r2.body);
}

#[tokio::test]
#[serial_test::serial]
async fn probes_speak_the_real_protocols() {
    let h = common::HiveHarness::configured().await;
    let jar = h.authorized_jar().await;

    // A REAL websocket handshake against a real WS route completes with 101 and an open socket.
    let ws = h.ws_probe(&format!("/api/tasks/stream/ws?project_id={}", uuid::Uuid::new_v4()), Some(&jar)).await;
    assert_eq!(ws.status, 101, "a valid handshake on a real WS route must upgrade");
    assert!(ws.upgraded, "tokio-tungstenite must report an established connection");

    // A REAL SSE request returns 200 + text/event-stream, and the probe must NOT hang on the
    // endless body.
    let sse = tokio::time::timeout(std::time::Duration::from_secs(5),
        h.sse_probe("/api/events", Some(&jar))).await
        .expect("sse_probe must not consume the endless body");
    assert_eq!(sse.status, 200);
    assert_eq!(sse.content_type.as_deref(), Some("text/event-stream"));
}

#[tokio::test]
#[serial_test::serial]
async fn profile_mocks_are_keyed_by_the_exact_generated_candidate_jwt() {
    let h = common::HiveHarness::configured().await;
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    h.mock_hive_oauth("code-a", "access-a", "refresh-a", first).await;
    h.mock_hive_oauth("code-b", "access-b", "refresh-b", second).await;

    // The access-token argument is a stable LABEL. Every path derives the same complete JWT.
    let jwt_a = h.access_token_for_label("access-a");
    let jwt_b = h.access_token_for_label("access-b");
    assert_ne!(jwt_a, "access-a");
    assert_ne!(jwt_a, jwt_b);
    assert!(utils::jwt::extract_expiration(&jwt_a).unwrap() > chrono::Utc::now());
    assert_eq!(h.redeemed_access_token("code-a").await, jwt_a,
        "redeem must return the exact JWT used by profile matching");
    assert_eq!(h.profile_subject_for("access-a").await, first);
    assert_eq!(h.profile_subject_for("access-b").await, second);
}

#[tokio::test]
#[serial_test::serial]
async fn restart_reuses_the_same_assets_dir_and_database() {
    let h = common::HiveHarness::configured().await;
    let project_id = h.seed_project("restart-probe", &[]).await;
    let old_generation = h.server_generation();
    let h = h.restart().await;
    assert_eq!(h.last_completed_server_generation(), Some(old_generation),
        "restart must record the old generation only after its serve JoinHandle completes");
    assert_eq!(h.server_generation(), old_generation + 1,
        "the replacement server is a new generation over the same persisted state");
    let mut jar = h.authorized_jar().await;
    let res = h.get_with("/api/projects", &mut jar).await;
    res.assert_registered();
    assert!(res.body.contains(&project_id.to_string()),
        "restart must reuse the same sqlite file; body: {}", res.body);
}

#[tokio::test]
#[serial_test::serial]
async fn resp_preserves_all_repeated_headers() {
    let h = common::HiveHarness::configured().await;
    h.mock_redirect("/header-probe", "/target", &["probe=a; HttpOnly", "other=b"]).await;
    let mut jar = common::CookieJar::new();
    let res = h.get_no_redirect("/header-probe", &mut jar).await;
    assert_eq!(res.status, 302);
    assert_eq!(res.headers.get_all(reqwest::header::SET_COOKIE).iter().count(), 2);
    assert_eq!(res.set_cookie.len(), 2);
}

#[tokio::test]
#[serial_test::serial]
async fn no_redirect_preserves_location() {
    let h = common::HiveHarness::configured().await;
    h.mock_redirect("/location-probe", "/target", &[]).await;
    let mut jar = common::CookieJar::new();
    let res = h.get_no_redirect("/location-probe", &mut jar).await;
    assert_eq!(res.status, 302);
    assert_eq!(res.location(), Some("/target"));
}

#[tokio::test]
#[serial_test::serial]
async fn priority_one_outage_overrides_signal_and_record_the_exact_request() {
    let h = common::HiveHarness::configured().await;
    let owner = uuid::Uuid::new_v4();
    h.mock_hive_oauth("code-a", "access-a", "refresh-a", owner).await;
    let reached = h.mock_hive_failure("POST", "/v1/tokens/refresh", 503).await;
    // Drive the real request in a spawned task; the signal fires from Wiremock's responder.
    let request = spawn_real_refresh_request(&h);
    tokio::time::timeout(std::time::Duration::from_secs(2), reached)
        .await.expect("refresh never reached Wiremock").unwrap();
    assert_eq!(h.hive_request_count("POST", "/v1/tokens/refresh").await, 1);
    request.abort();
    let _ = request.await;
}

async fn spawn_real_refresh_request(h: &common::HiveHarness) -> tokio::task::JoinHandle<()> {
    let addr = h.addr();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(format!("http://{}/api/organizations", addr))
            .send()
            .await;
    })
}
