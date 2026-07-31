mod common;

#[tokio::test]
#[serial_test::serial]
async fn configured_hive_returns_success() {
    let h = common::HiveHarness::configured().await;
    h.mock_json(
        "GET",
        "/v1/swarm/templates",
        200,
        serde_json::json!({"templates": []}),
    )
    .await;
    let res = h
        .get("/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000")
        .await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);
    assert!(res.body.contains("\"success\":true"), "body: {}", res.body);
}

#[tokio::test]
#[serial_test::serial]
async fn absent_hive_is_registered_and_not_a_500() {
    let h = common::HiveHarness::hive_absent().await;
    let res = h
        .get("/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000")
        .await;
    res.assert_registered();
    assert_ne!(
        res.status, 500,
        "hive-absent must be the specific HiveNotConfigured 503 (task 401), never an unhandled 500"
    );
}
