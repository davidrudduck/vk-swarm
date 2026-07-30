mod common;

#[tokio::test]
#[serial_test::serial]
async fn harness_serves_a_configured_hive() {
    let h = common::HiveHarness::configured().await;
    h.mock_json("GET", "/v1/organizations", 200, serde_json::json!({"organizations": []}))
        .await;
    let res = h.get("/api/organizations").await;
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
    assert_ne!(res.status, 404, "route must be registered");
    assert_ne!(res.status, 500, "absent hive is not a server error");
}
