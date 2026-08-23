// This binary exercises only the protocol probes (ws_probe/sse_probe); the
// shared harness's HTTP Resp helpers would otherwise be dead code here.
#[allow(dead_code)]
mod common;

fn protected_ws(id: uuid::Uuid) -> Vec<(String, u16)> {
    vec![
        (format!("/api/tasks/stream/ws?project_id={id}"), 101),
        (format!("/api/drafts/stream/ws?project_id={id}"), 101),
        (format!("/api/task-attempts/{id}/diff/ws"), 500), // RemoteAttemptNeeded + required Extension<TaskAttempt> -> MissingExtension (see census note)
        (format!("/api/task-attempts/by-task-id/{id}/diff/ws"), 404),
        (
            format!("/api/execution-processes/stream/ws?task_attempt_id={id}"),
            101,
        ),
        (format!("/api/execution-processes/{id}/raw-logs/ws"), 404),
        (
            format!("/api/execution-processes/{id}/normalized-logs/ws"),
            404,
        ),
        (format!("/api/logs/{id}/live"), 404),
        (format!("/api/terminal/ws/{id}"), 400),
    ]
}

fn direct_connection_ws(id: uuid::Uuid) -> [String; 3] {
    [
        format!("/api/task-attempts/{id}/diff/ws"),
        format!("/api/execution-processes/{id}/raw-logs/ws"),
        format!("/api/logs/{id}/live"),
    ]
}

fn with_token(path: &str, token: &str) -> String {
    format!(
        "{path}{}token={token}",
        if path.contains('?') { "&" } else { "?" }
    )
}

#[tokio::test]
#[serial_test::serial]
async fn every_protected_stream_rejects_anonymously_before_lookup_or_upgrade() {
    let h = common::HiveHarness::configured().await;
    let id = uuid::Uuid::new_v4();
    for (path, _) in protected_ws(id) {
        let res = h.ws_probe(&path, None).await;
        assert_eq!(
            res.status, 401,
            "{path}: anonymous must be 401 (404 means lookup ran; 101 means upgrade ran)"
        );
        assert!(!res.upgraded, "{path}: upgraded anonymously");
    }
    let sse = h.sse_probe("/api/events", None).await;
    assert_eq!(sse.status, 401);
    assert_ne!(sse.content_type.as_deref(), Some("text/event-stream"));
}

#[tokio::test]
#[serial_test::serial]
async fn an_authorized_browser_reaches_every_protected_stream() {
    let h = common::HiveHarness::configured().await;
    let jar = h.authorized_jar().await;
    let id = uuid::Uuid::new_v4();
    for (path, expected) in protected_ws(id) {
        let res = h.ws_probe(&path, Some(&jar)).await;
        assert_eq!(res.status, expected, "{path}: browser boundary result");
    }
    let sse = h.sse_probe("/api/events", Some(&jar)).await;
    assert_eq!(sse.status, 200);
    assert_eq!(sse.content_type.as_deref(), Some("text/event-stream"));
}

#[tokio::test]
#[serial_test::serial]
async fn browser_session_wins_over_an_irrelevant_bad_token_on_direct_streams() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let jar = h.authorized_jar().await;
    let id = uuid::Uuid::new_v4();
    let proxy = mint_proxy_token(SECRET, node_id);

    for path in direct_connection_ws(id) {
        let browser_only = h.ws_probe(&path, Some(&jar)).await;
        assert_ne!(
            browser_only.status, 401,
            "{path}: browser session must pass auth"
        );
        for bad_token in ["garbage", proxy.as_str()] {
            let with_bad_token = h.ws_probe(&with_token(&path, bad_token), Some(&jar)).await;
            assert_eq!(
                with_bad_token.status, browser_only.status,
                "{path}: a valid browser is the chosen OR branch; an irrelevant malformed or wrong-audience query token must not turn it into browser AND token"
            );
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn direct_logs_and_direct_diff_accept_only_a_scoped_connection_token() {
    let _guard = with_connection_secret(SECRET);
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let scoped = mint_connection_token(SECRET, node_id, Some(id));
    let wrong_scope = mint_connection_token(SECRET, node_id, Some(uuid::Uuid::new_v4()));
    let unscoped = mint_connection_token(SECRET, node_id, None);
    let wrong_node = mint_connection_token(SECRET, uuid::Uuid::new_v4(), Some(id));
    let proxy = mint_proxy_token(SECRET, node_id);

    for path in direct_connection_ws(id) {
        assert_eq!(h.ws_probe(&path, None).await.status, 401, "{path}: missing");
        assert_eq!(
            h.ws_probe(&with_token(&path, "garbage"), None).await.status,
            401,
            "{path}: malformed token must stop before lookup"
        );
        assert_eq!(
            h.ws_probe(&with_token(&path, &wrong_scope), None)
                .await
                .status,
            401,
            "{path}: wrong resource scope must stop before lookup"
        );
        assert_eq!(
            h.ws_probe(&with_token(&path, &unscoped), None).await.status,
            401,
            "{path}: absent resource scope must stop before lookup"
        );
        assert_eq!(
            h.ws_probe(&with_token(&path, &wrong_node), None)
                .await
                .status,
            401,
            "{path}: wrong target node must stop before lookup"
        );
        assert_eq!(
            h.ws_probe(&with_token(&path, &proxy), None).await.status,
            401,
            "{path}: node_proxy must never open direct logs or diff"
        );
        let accepted = h.ws_probe(&with_token(&path, &scoped), None).await;
        assert_ne!(
            accepted.status, 401,
            "{path}: correctly scoped connection token must pass auth; body status {}",
            accepted.status
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn browser_only_streams_reject_both_non_browser_token_classes() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let conn = mint_connection_token(SECRET, node_id, Some(id));
    let proxy = mint_proxy_token(SECRET, node_id);
    let direct = direct_connection_ws(id);
    for (path, _) in protected_ws(id) {
        if direct.contains(&path) {
            continue;
        }
        assert_eq!(
            h.ws_probe(&with_token(&path, &conn), None).await.status,
            401,
            "{path}: connection query token is not an alternative here"
        );
        assert_eq!(
            h.ws_probe(&with_token(&path, &proxy), None).await.status,
            401,
            "{path}: proxy query token is not an alternative here"
        );
    }
}

// ============================================================================
// Test-local fixtures (NOT in the shared harness, per the task contract).
// ============================================================================

/// STANDARD base64 of the same 32 fixed bytes as `test_secret()` in
/// `services/src/services/connection_token.rs`.
const SECRET: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

/// Keep `VK_CONNECTION_TOKEN_SECRET` set for the guard's lifetime so any
/// environment-sensitive component constructed inside the test sees the secret.
struct ConnectionSecretEnvGuard;

impl ConnectionSecretEnvGuard {
    fn set(secret: &str) -> Self {
        unsafe { std::env::set_var("VK_CONNECTION_TOKEN_SECRET", secret) };
        ConnectionSecretEnvGuard
    }
}

impl Drop for ConnectionSecretEnvGuard {
    fn drop(&mut self) {
        unsafe { std::env::remove_var("VK_CONNECTION_TOKEN_SECRET") };
    }
}

fn with_connection_secret(secret: &str) -> ConnectionSecretEnvGuard {
    ConnectionSecretEnvGuard::set(secret)
}

/// Mint a `connection`-audience token exactly from `ConnectionTokenClaims`.
fn mint_connection_token(
    secret: &str,
    node_id: uuid::Uuid,
    resource: Option<uuid::Uuid>,
) -> String {
    let now = chrono::Utc::now();
    let claims = services::services::connection_token::ConnectionTokenClaims {
        sub: uuid::Uuid::new_v4(),
        node_id,
        assignment_id: uuid::Uuid::new_v4(),
        execution_process_id: resource,
        iat: now.timestamp(),
        exp: (now + chrono::Duration::minutes(15)).timestamp(),
        aud: "connection".into(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_base64_secret(secret).unwrap(),
    )
    .unwrap()
}

/// Mint a `node_proxy`-audience token exactly from `ProxyTokenClaims`.
fn mint_proxy_token(secret: &str, target: uuid::Uuid) -> String {
    let now = chrono::Utc::now();
    let claims = services::services::connection_token::ProxyTokenClaims {
        sub: uuid::Uuid::new_v4().to_string(),
        node_id: target.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::minutes(15)).timestamp(),
        aud: "node_proxy".into(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_base64_secret(secret).unwrap(),
    )
    .unwrap()
}
