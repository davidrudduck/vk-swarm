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
    // mock_hive_oauth mounts /v1/oauth/web/init with up_to_n_times(1); a second
    // initiation would find no mock and fail closed (no cookie, no row). Two
    // initiations therefore require two mounted mocks.
    h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    h.mock_hive_oauth("code-2", "acc-2", "ref-2", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let mut b = common::CookieJar::fresh();
    let ra = h
        .post_with(
            "/api/auth/handoff/init",
            serde_json::json!({"provider":"github","return_to":"/"}),
            &mut a,
        )
        .await;
    assert_eq!(ra.status, 200, "browser A initiation failed: {}", ra.body);
    let rb = h
        .post_with(
            "/api/auth/handoff/init",
            serde_json::json!({"provider":"github","return_to":"/"}),
            &mut b,
        )
        .await;
    assert_eq!(rb.status, 200, "browser B initiation failed: {}", rb.body);
    let a_raw = a
        .get("vks_browser_binding")
        .expect("browser A must receive a binding cookie");
    let b_raw = b
        .get("vks_browser_binding")
        .expect("browser B must receive a binding cookie");
    assert_ne!(a_raw, b_raw);
}

#[tokio::test]
#[serial_test::serial]
async fn initiation_persists_the_handoff_behind_the_epoch_fence() {
    use deployment::Deployment;

    let h = common::HiveHarness::configured().await;
    let handoff_id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;

    // Hold the epoch fence from the outside. The handler may finish its Hive
    // I/O unfenced, but its create_handoff must queue behind the fence: the
    // handoff row may not exist while the fence is held, and must exist once
    // it is released.
    let epoch = h.deployment().browser_auth_epoch().clone();
    let fence = epoch.lock().await;

    let addr = h.addr();
    let pool = h.pool().clone();
    let request = tokio::spawn(async move {
        reqwest::Client::new()
            .post(format!("http://{addr}/api/auth/handoff/init"))
            .json(&serde_json::json!({"provider":"github","return_to":"/"}))
            .send()
            .await
            .expect("initiation request must complete")
    });

    // Wait until the request has certainly passed its Hive I/O (the init mock
    // was served), then give the handler every opportunity to mis-insert.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2_000);
    while h.hive_request_count("POST", "/v1/oauth/web/init").await < 1 {
        assert!(
            std::time::Instant::now() < deadline,
            "initiation never reached the Hive mock"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let row: Option<(String,)> =
        sqlx::query_as("SELECT binding_hash FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(handoff_id)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(
        row.is_none(),
        "handoff row appeared while the epoch fence was held"
    );

    drop(fence);
    let res = request.await.unwrap();
    assert_eq!(
        res.status(),
        200,
        "initiation must succeed once the fence opens"
    );

    let state: String =
        sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(handoff_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, "pending");
}
