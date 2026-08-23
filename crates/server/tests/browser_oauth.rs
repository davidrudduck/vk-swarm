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

/// Drive initiation in `jar` and return (handoff_id, callback_path).
async fn start_login(
    h: &common::HiveHarness,
    jar: &mut common::CookieJar,
    handoff_id: uuid::Uuid,
) -> String {
    let res = h
        .post_with(
            "/api/auth/handoff/init",
            serde_json::json!({"provider":"github","return_to":"/"}),
            jar,
        )
        .await;
    assert_eq!(res.status, 200, "body: {}", res.body);
    format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code=code-1")
}

#[tokio::test]
#[serial_test::serial]
async fn a_copied_callback_url_cannot_be_completed_in_another_browser() {
    let h = common::HiveHarness::configured().await;
    let id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let mut b = common::CookieJar::fresh();
    let cb = start_login(&h, &mut a, id).await;
    let redeems = h.hive_request_count("POST", "/v1/oauth/web/redeem").await;

    // Browser B copies the URL. B has no binding cookie at all.
    let stolen = h.get_with(&cb, &mut b).await;
    assert_eq!(stolen.status, 400, "body: {}", stolen.body);
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        redeems,
        "a cookieless wrong-browser callback must not reach Hive redemption"
    );
    assert!(
        stolen
            .set_cookie
            .iter()
            .all(|c| !c.starts_with("vks_browser_session=")),
        "a wrong-browser callback must not mint a session"
    );
    let state: String =
        sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(id)
            .fetch_one(h.pool())
            .await
            .unwrap();
    assert_eq!(
        state, "pending",
        "the rightful handoff must NOT have been consumed"
    );

    // A second stolen attempt smuggles browser A's RAW binding token as a query
    // parameter. The binding secret is read only from request headers, so even
    // the genuine secret is inert in the URL.
    let raw_binding = a
        .header_value()
        .expect("browser A must hold a binding cookie")
        .split(';')
        .find_map(|part| part.trim().strip_prefix("vks_browser_binding="))
        .expect("binding cookie missing from browser A's Cookie header")
        .to_string();
    let smuggled_url = format!("{cb}&vks_browser_binding={raw_binding}");
    let smuggled = h.get_with(&smuggled_url, &mut b).await;
    assert_eq!(
        smuggled.status, 400,
        "a query-parameter binding token must not complete the handoff: {}",
        smuggled.body
    );
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        redeems,
        "a query-parameter binding token must not reach Hive redemption"
    );
    let state_after_smuggle: String =
        sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(id)
            .fetch_one(h.pool())
            .await
            .unwrap();
    assert_eq!(
        state_after_smuggle, "pending",
        "even a raw-token query smuggle must not consume the handoff"
    );

    // The rightful browser still completes -- exactly one redemption for the pair.
    let ok = h.get_with(&cb, &mut a).await;
    assert_eq!(ok.status, 200, "body: {}", ok.body);
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        redeems + 1,
        "only the rightful completion may redeem"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn a_forged_binding_cookie_does_not_consume_the_handoff() {
    let h = common::HiveHarness::configured().await;
    let id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    let mut forged = common::CookieJar::fresh();
    forged.insert("vks_browser_binding", "not-the-real-secret");
    let redeems = h.hive_request_count("POST", "/v1/oauth/web/redeem").await;
    assert_eq!(h.get_with(&cb, &mut forged).await.status, 400);
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        redeems,
        "a forged binding cookie must not burn the one-time Hive code"
    );
    let state: String =
        sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(id)
            .fetch_one(h.pool())
            .await
            .unwrap();
    assert_eq!(state, "pending");
}

#[tokio::test]
#[serial_test::serial]
async fn replaying_a_completed_callback_is_rejected() {
    let h = common::HiveHarness::configured().await;
    let id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    assert_eq!(h.get_with(&cb, &mut a).await.status, 200);
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        1,
        "one successful completion is exactly one redemption"
    );
    let replay = h.get_with(&cb, &mut a).await;
    assert_eq!(
        replay.status, 400,
        "a claimed handoff is terminal: {}",
        replay.body
    );
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        1,
        "a replayed callback must not redeem again"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn an_expired_handoff_cannot_be_completed() {
    let h = common::HiveHarness::configured().await;
    let id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    // Age the row past its TTL through the DB rather than by sleeping.
    sqlx::query("UPDATE browser_oauth_handoffs SET expires_at = created_at WHERE handoff_id = ?")
        .bind(id)
        .execute(h.pool())
        .await
        .unwrap();
    let redeems = h.hive_request_count("POST", "/v1/oauth/web/redeem").await;
    assert_eq!(h.get_with(&cb, &mut a).await.status, 400);
    assert_eq!(
        h.hive_request_count("POST", "/v1/oauth/web/redeem").await,
        redeems,
        "an expired handoff must not reach Hive redemption"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn completion_drops_the_epoch_fence_before_hive_redemption() {
    use deployment::Deployment;

    let h = common::HiveHarness::configured().await;
    let id = h
        .mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4())
        .await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;

    // Priority-1 delayed responder: signals the moment the redeem request ARRIVES,
    // then hangs for 60s, so redemption is provably still in flight afterwards.
    let redeem_arrived = h.mock_hive_delayed("POST", "/v1/oauth/web/redeem").await;

    let addr = h.addr();
    let cookie = a
        .header_value()
        .expect("browser A must hold a binding cookie");
    let request = tokio::spawn(async move {
        reqwest::Client::new()
            .get(format!("http://{addr}{cb}"))
            .header(reqwest::header::COOKIE, cookie)
            .send()
            .await
            .expect("callback request must be issued")
    });

    tokio::time::timeout(std::time::Duration::from_secs(2), redeem_arrived)
        .await
        .expect("redemption never reached the Hive mock")
        .expect("delayed-responder signal channel closed");

    // Redemption is in flight, so the claim must already be committed and its
    // epoch guard released: a mutant holding the fence across Hive I/O makes
    // this try_lock fail while the delayed response is pending.
    let guard = h
        .deployment()
        .browser_auth_epoch()
        .try_lock()
        .expect("epoch fence is still held while Hive redemption is in flight");
    drop(guard);

    request.abort();
    let _ = request.await;
}
