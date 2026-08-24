mod common;

#[tokio::test]
#[serial_test::serial]
async fn proxy_http_routes_accept_browser_or_node_proxy_but_reject_missing_and_connection() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let proxy = mint_proxy_token(SECRET, node_id);
    let wrong_target_proxy = mint_proxy_token(SECRET, uuid::Uuid::new_v4());
    let conn = mint_connection_token(SECRET, node_id, Some(id));
    let paths = [
        format!("/api/projects/by-remote-id/{id}/branches"),
        format!("/api/task-attempts/by-task-id/{id}/branch-status"),
    ];

    for path in paths {
        assert_eq!(
            h.get(&path).await.status,
            401,
            "{path}: missing credential must stop before lookup"
        );
        assert_eq!(
            h.get_with_headers(&path, &[("authorization", "Bearer garbage")])
                .await
                .status,
            401,
            "{path}: invalid proxy token must stop before lookup"
        );
        assert_eq!(
            h.get_with_headers(&path, &[("authorization", &format!("Bearer {conn}"))])
                .await
                .status,
            401,
            "{path}: connection audience must not open proxy HTTP"
        );
        assert_eq!(
            h.get_with_headers(
                &path,
                &[("authorization", &format!("Bearer {wrong_target_proxy}"))]
            )
            .await
            .status,
            401,
            "{path}: wrong target node must stop before lookup"
        );
        assert_ne!(
            h.get_with_headers(&path, &[("authorization", &format!("Bearer {proxy}"))])
                .await
                .status,
            401,
            "{path}: valid node_proxy must pass the auth boundary"
        );

        let mut jar = h.authorized_jar().await;
        assert_ne!(
            h.get_with(&path, &mut jar).await.status,
            401,
            "{path}: browser session must bypass the inner proxy-token requirement"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_tokens_fail_every_direct_log_and_direct_diff_route() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let proxy = mint_proxy_token(SECRET, node_id);
    for path in [
        format!("/api/logs/{id}/live"),
        format!("/api/execution-processes/{id}/raw-logs/ws"),
        format!("/api/task-attempts/{id}/diff/ws"),
    ] {
        assert_eq!(
            h.ws_probe(&format!("{path}?token={proxy}"), None)
                .await
                .status,
            401,
            "{path}: node_proxy must never open a direct stream"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn by_task_id_diff_is_browser_only_not_either_token_alternative() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let path = format!("/api/task-attempts/by-task-id/{id}/diff/ws");
    let conn = mint_connection_token(SECRET, node_id, Some(id));
    let proxy = mint_proxy_token(SECRET, node_id);
    assert_eq!(h.ws_probe(&path, None).await.status, 401);
    assert_eq!(
        h.ws_probe(&format!("{path}?token={conn}"), None)
            .await
            .status,
        401,
        "by-task-id diff is not the production direct connection-token URL"
    );
    assert_eq!(
        h.ws_probe(&format!("{path}?token={proxy}"), None)
            .await
            .status,
        401,
        "proxy query token must not open a WebSocket"
    );
    assert_eq!(
        h.ws_probe_with_headers(
            &path,
            None,
            &[("authorization", &format!("Bearer {proxy}"))]
        )
        .await
        .status,
        401,
        "proxy bearer token must not open a WebSocket"
    );

    let jar = h.authorized_jar().await;
    assert_eq!(
        h.ws_probe(&path, Some(&jar)).await.status,
        404,
        "browser passes auth; random task id is looked up only afterwards"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn disabled_validator_has_no_anonymous_or_token_fallback() {
    let h = common::HiveHarness::configured().await;
    let path = format!(
        "/api/projects/by-remote-id/{}/branches",
        uuid::Uuid::new_v4()
    );
    assert_eq!(h.get(&path).await.status, 401);
    assert_eq!(
        h.get_with_headers(
            &path,
            &[(
                "authorization",
                &format!("Bearer {}", mint_proxy_token(SECRET, uuid::Uuid::new_v4()))
            )]
        )
        .await
        .status,
        401
    );
}

/// STANDARD base64 of the same 32 fixed bytes as
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
