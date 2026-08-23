mod common;

#[tokio::test]
#[serial_test::serial]
async fn configured_hive_returns_success() {
    let h = common::HiveHarness::configured().await;
    h.mock_json("GET", "/v1/nodes", 200, serde_json::json!([]))
        .await;
    let mut jar = h.authorized_jar().await;
    let res = h
        .get_with("/api/nodes?organization_id=00000000-0000-0000-0000-000000000000", &mut jar)
        .await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);
    assert!(res.body.contains("\"success\":true"), "body: {}", res.body);
}

#[tokio::test]
#[serial_test::serial]
async fn absent_hive_is_registered_and_not_a_500() {
    let h = common::HiveHarness::hive_absent().await;
    let mut jar = h.authorized_jar().await;
    let res = h
        .get_with("/api/nodes?organization_id=00000000-0000-0000-0000-000000000000", &mut jar)
        .await;
    res.assert_registered();
    assert_eq!(
        res.status, 503,
        "hive-absent must be the specific HiveNotConfigured 503 (task 401), never an unhandled 500 \
         and never a silently-different status; body: {}",
        res.body
    );

    // CONTRACT PIN for the frontend's isHiveNotConfigured() guard.
    //
    // Status alone cannot identify this case: `RemoteClientError::Http` forwards the upstream
    // status verbatim (`error.rs`: `StatusCode::from_u16(*status)`), so a configured-but-DOWN
    // hive also yields 503. The frontend therefore discriminates on this message prefix
    // (`frontend/src/lib/api/utils.ts`, HIVE_NOT_CONFIGURED_CODE). If the message shape changes,
    // this test must fail HERE — otherwise the guard goes silently dead and a hive OUTAGE gets
    // rendered as "not connected to a hive" with retries suppressed.
    assert!(
        res.body.contains("HiveNotConfigured"),
        "the 503 body must carry the HiveNotConfigured discriminator the frontend matches on; \
         body: {}",
        res.body
    );
}
