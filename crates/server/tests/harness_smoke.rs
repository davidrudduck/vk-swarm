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
    let res = h.get("/api/organizations").await;
    res.assert_registered();
    assert_ne!(res.status, 500, "absent hive is not a server error");
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
