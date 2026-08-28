#[allow(dead_code)]
mod common;

use common::{CookieJar, HiveHarness, Resp};
use serde_json::json;
use uuid::Uuid;

const ACCESS_LABEL: &str = "SENTINEL-ACCESS-8f31c0d2";
const REFRESH_SENTINEL: &str = "SENTINEL-REFRESH-4b7ae19f";

fn assert_clean(label: &str, haystack: &str, access_jwt: &str) {
    assert!(
        !haystack.contains(access_jwt),
        "{label} leaked access JWT: {haystack}"
    );
    assert!(
        !haystack.contains(REFRESH_SENTINEL),
        "{label} leaked refresh sentinel: {haystack}"
    );
}

fn scan_logs(label: &str, access_jwt: &str, logs_contain: impl Fn(&str) -> bool) {
    assert!(!logs_contain(access_jwt), "{label} logs leaked access JWT");
    assert!(
        !logs_contain(REFRESH_SENTINEL),
        "{label} logs leaked refresh sentinel"
    );
}

fn scan_resp(
    label: &str,
    resp: &Resp,
    access_jwt: &str,
    jar: &CookieJar,
    logs_contain: impl Fn(&str) -> bool,
) {
    assert_clean(&format!("{label} body"), &resp.body, access_jwt);
    for (name, value) in resp.headers.iter() {
        assert_clean(
            &format!("{label} header {name}"),
            value.to_str().unwrap_or("<bin>"),
            access_jwt,
        );
    }
    for cookie in &resp.set_cookie {
        assert_clean(&format!("{label} set-cookie"), cookie, access_jwt);
    }
    if let Some(cookie) = jar.header_value() {
        assert_clean(&format!("{label} jar"), &cookie, access_jwt);
    }
    scan_logs(label, access_jwt, logs_contain);
}

async fn login(
    h: &HiveHarness,
    subject: uuid::Uuid,
    app_code: &str,
    access_label: &str,
    refresh: &str,
    logs_contain: impl Fn(&str) -> bool,
) -> CookieJar {
    let handoff_id = h
        .mock_hive_oauth(app_code, access_label, refresh, subject)
        .await;
    let mut jar = CookieJar::fresh();
    let init = h
        .post_with(
            "/api/auth/handoff/init",
            serde_json::json!({"provider":"github","return_to":"/"}),
            &mut jar,
        )
        .await;
    assert_eq!(init.status, 200, "init body: {}", init.body);
    let access_jwt = h.access_token_for_label(ACCESS_LABEL);
    scan_resp(
        &format!("init {app_code}"),
        &init,
        &access_jwt,
        &jar,
        &logs_contain,
    );
    let complete = h
        .get_no_redirect(
            &format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code={app_code}"),
            &mut jar,
        )
        .await;
    scan_resp(
        &format!("complete {app_code}"),
        &complete,
        &access_jwt,
        &jar,
        &logs_contain,
    );
    assert!(
        matches!(complete.status, 200 | 204 | 302),
        "complete {app_code} status: {} body: {}",
        complete.status,
        complete.body
    );
    jar
}

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn scanner_detects_deliberate_jwt_log_leak() {
    let h = HiveHarness::configured().await;
    let access_jwt = h.access_token_for_label(ACCESS_LABEL);
    tracing::error!("{access_jwt}");
    assert!(logs_contain(&access_jwt));
}

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn sentinel_oauth_surfaces_do_not_disclose_tokens() {
    let h = HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let access_jwt = h.access_token_for_label(ACCESS_LABEL);
    let mut other_jar = login(
        &h,
        owner,
        "code-a",
        "other-access",
        "other-refresh",
        logs_contain,
    )
    .await;
    let mut sentinel_jar = login(
        &h,
        owner,
        "code-1",
        ACCESS_LABEL,
        REFRESH_SENTINEL,
        logs_contain,
    )
    .await;

    for path in [
        "/api/auth/state",
        "/api/info",
        "/api/auth/status",
        "/api/projects",
    ] {
        let resp = h.get_with(path, &mut sentinel_jar).await;
        scan_resp(path, &resp, &access_jwt, &sentinel_jar, logs_contain);
    }

    let browser_logout = h
        .post_with("/api/auth/browser/logout", json!({}), &mut sentinel_jar)
        .await;
    assert!(matches!(browser_logout.status, 200 | 204));
    scan_resp(
        "browser logout",
        &browser_logout,
        &access_jwt,
        &sentinel_jar,
        logs_contain,
    );

    let disconnect = h
        .post_with("/api/auth/logout", json!({}), &mut other_jar)
        .await;
    assert!(matches!(disconnect.status, 200 | 204));
    assert_ne!(disconnect.status, 401);
    scan_resp(
        "disconnect",
        &disconnect,
        &access_jwt,
        &other_jar,
        logs_contain,
    );
    scan_logs("disconnect", &access_jwt, logs_contain);
}

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn different_owner_complete_does_not_disclose_or_write() {
    let h = HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let _owner_jar = login(
        &h,
        owner,
        "code-a",
        "other-access",
        "other-refresh",
        logs_contain,
    )
    .await;
    let access_jwt = h.access_token_for_label(ACCESS_LABEL);
    let handoff_id = h
        .mock_hive_oauth(
            "code-intruder",
            ACCESS_LABEL,
            REFRESH_SENTINEL,
            Uuid::new_v4(),
        )
        .await;
    let mut jar = CookieJar::fresh();
    let init = h
        .post_with(
            "/api/auth/handoff/init",
            json!({"provider":"github","return_to":"/"}),
            &mut jar,
        )
        .await;
    let complete = h
        .get_no_redirect(
            &format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code=code-intruder"),
            &mut jar,
        )
        .await;
    assert_eq!(complete.status, 400);
    assert!(complete.body.contains("owned by a different account"));
    scan_resp("intruder init", &init, &access_jwt, &jar, logs_contain);
    scan_resp(
        "intruder complete",
        &complete,
        &access_jwt,
        &jar,
        logs_contain,
    );
    let stored_owner: Vec<u8> = sqlx::query_scalar("SELECT hive_user_id FROM node_owner")
        .fetch_one(h.pool())
        .await
        .unwrap();
    assert_eq!(Uuid::from_slice(&stored_owner).unwrap(), owner);
    assert!(h.credentials_path().exists());
}

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn upstream_5xx_body_with_sentinels_is_not_forwarded() {
    let h = HiveHarness::configured().await;
    let access_jwt = h.access_token_for_label(ACCESS_LABEL);
    let handoff_id = uuid::Uuid::new_v4();
    h.mock_json(
        "POST",
        "/v1/oauth/web/init",
        200,
        serde_json::json!({"handoff_id": handoff_id, "authorize_url": "https://github.com/login/oauth/authorize"}),
    )
    .await;
    h.mock_json(
        "POST",
        "/v1/oauth/web/redeem",
        500,
        serde_json::json!({"access_token": access_jwt, "refresh_token": REFRESH_SENTINEL, "error": "upstream"}),
    )
    .await;
    let mut jar = CookieJar::fresh();
    let init = h
        .post_with(
            "/api/auth/handoff/init",
            json!({"provider":"github","return_to":"/"}),
            &mut jar,
        )
        .await;
    let complete = h
        .get_no_redirect(
            &format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code=code-5xx"),
            &mut jar,
        )
        .await;
    scan_resp("5xx init", &init, &access_jwt, &jar, logs_contain);
    scan_resp("5xx complete", &complete, &access_jwt, &jar, logs_contain);
}
