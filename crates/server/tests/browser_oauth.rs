mod common;

#[tokio::test]
#[serial_test::serial]
async fn initiation_issues_a_binding_cookie_and_persists_only_its_hash() {
    let h = common::HiveHarness::configured().await;
    let handoff_id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut jar = common::CookieJar::new();

    let res = h
        .post_with(
            "/api/auth/handoff/init",
            serde_json::json!({"provider": "github", "return_to": "/"}),
            &mut jar,
        )
        .await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);

    let line = res
        .set_cookie
        .iter()
        .find(|c| c.starts_with("vks_browser_binding="))
        .expect("no binding cookie issued");
    assert!(line.contains("HttpOnly"), "{line}");
    assert!(line.contains("SameSite=Lax"), "{line}");
    assert!(line.contains("Path=/"), "{line}");
    assert!(line.contains("Max-Age=600"), "{line}");
    assert!(
        !line.contains("Secure"),
        "D9: no Secure on the plain-HTTP LAN boundary: {line}"
    );

    let raw = jar
        .get("vks_browser_binding")
        .expect("jar did not store the cookie")
        .to_string();
    assert!(
        !res.body.contains(&raw),
        "binding secret leaked into the response body"
    );

    let (stored, created, expires): (String, i64, i64) = sqlx::query_as(
        "SELECT binding_hash, created_at, expires_at FROM browser_oauth_handoffs WHERE handoff_id = ?")
        .bind(handoff_id).fetch_one(h.pool()).await.unwrap();
    assert_eq!(
        stored,
        server::auth::seams::hash_token(&raw),
        "only the hash may be stored"
    );
    assert_ne!(stored, raw);
    assert_eq!(expires - created, 600_000, "exactly ten minutes");

    let state: String =
        sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(handoff_id)
            .fetch_one(h.pool())
            .await
            .unwrap();
    assert_eq!(state, "pending");
}

#[tokio::test]
#[serial_test::serial]
async fn two_browsers_get_two_different_binding_secrets() {
    let h = common::HiveHarness::configured().await;
    h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let mut b = common::CookieJar::fresh();
    h.post_with(
        "/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}),
        &mut a,
    )
    .await;
    h.post_with(
        "/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}),
        &mut b,
    )
    .await;
    assert_ne!(a.get("vks_browser_binding"), b.get("vks_browser_binding"));
}
