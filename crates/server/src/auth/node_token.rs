//! Route-scoped alternative credentials.
//!
//! The node accepts exactly two kinds of non-browser credential, and they are NOT
//! interchangeable. A single "any valid node token" predicate would let a hive-issued
//! log-streaming token open the node-to-node proxy surface (project files, follow-ups, PR
//! creation) and let a node proxy token open live execution logs -- a privilege widening across
//! route classes that no criterion asks for. The two classes are already separated at the JWT
//! `aud` claim by the validator (crates/services/src/services/connection_token.rs: `validate()`
//! sets audience "connection"; `validate_proxy_token()` sets "node_proxy"), so keeping them
//! apart here costs one extra function and closes the widening by construction.

use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use deployment::Deployment;
use services::services::connection_token::ConnectionTokenValidator;
use uuid::Uuid;

use crate::DeploymentImpl;
use crate::auth::session::resolve_browser_session;

/// Strict connection-credential predicate: the token must carry the `connection` audience, be
/// issued for THIS node, and be scoped to THIS exact resource. `None` (unscoped) is not wildcard
/// access, and a `node_proxy`-audience token never passes.
pub fn connection_token_is_valid_for_resource(
    validator: &ConnectionTokenValidator,
    token: Option<&str>,
    expected_node_id: Uuid,
    expected_resource_id: Uuid,
) -> bool {
    token.is_some_and(|t| {
        validator
            .validate_for_resource(t, expected_node_id, expected_resource_id)
            .is_ok()
    })
}

/// Strict proxy-credential predicate: the token must carry the `node_proxy` audience and target
/// THIS node. A `connection`-audience token never passes.
pub fn proxy_token_is_valid_for_node(
    validator: &ConnectionTokenValidator,
    token: Option<&str>,
    expected_node_id: Uuid,
) -> bool {
    token.is_some_and(|t| {
        validator
            .validate_proxy_for_node(t, expected_node_id)
            .is_ok()
    })
}

/// Extract the bearer credential from an `Authorization` header.
/// RFC 9110 §11.1: the auth-scheme is case-insensitive, so `Bearer`,
/// `bearer`, and `BEARER` are all accepted; the credential itself is not
/// modified.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") {
        Some(token)
    } else {
        None
    }
}

/// Extract the `token` query parameter (percent-decoded) from the request URI.
fn query_token(uri: &Uri) -> Option<String> {
    let query = uri.query()?;
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.into_owned())
}

/// Exactly the attempt-id direct diff plus raw/live direct logs. First resolve a browser session;
/// otherwise extract the nested node identity fail-closed:
/// `let runner = deployment.node_runner_context().ok_or(StatusCode::UNAUTHORIZED)?;`
/// `let expected_node_id = runner.node_id().await.ok_or(StatusCode::UNAUTHORIZED)?;`
/// then extract the route's sole UUID capture and `?token=` and call the
/// strict connection predicate. Insert BrowserSessionCtx on the browser branch. Never call next
/// for a missing/malformed/wrong-audience/wrong-node/unscoped/wrong-resource token.
pub async fn require_session_or_connection_token(
    State(deployment): State<DeploymentImpl>,
    Path(resource_id): Path<Uuid>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut request = request;

    // Browser branch first: independent of node-runner availability. Resolve, insert, and
    // return BEFORE touching node-runner state (compile-order contract).
    if let Some(ctx) = resolve_browser_session(&deployment.db().pool, request.headers()).await {
        request.extensions_mut().insert(ctx);
        return Ok(next.run(request).await);
    }

    // Non-browser branch: establish this node's own identity, failing closed at each layer.
    let runner = deployment
        .node_runner_context()
        .ok_or(StatusCode::UNAUTHORIZED)?; // outer Option<&NodeRunnerContext>
    let expected_node_id = runner.node_id().await.ok_or(StatusCode::UNAUTHORIZED)?; // inner async Option<Uuid>

    let token = query_token(request.uri());
    let validator = deployment.connection_token_validator();
    if connection_token_is_valid_for_resource(
        validator,
        token.as_deref(),
        expected_node_id,
        resource_id,
    ) {
        Ok(next.run(request).await)
    } else {
        tracing::warn!(
            resource_id = %resource_id,
            "rejecting request without a valid browser session or connection token"
        );
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// By-remote-id/by-task-id HTTP only, excluding diff. First resolve and insert BrowserSessionCtx;
/// otherwise obtain the current node ID and fail 401 if absent, require Authorization: Bearer,
/// and call only the strict proxy predicate. Query tokens and connection audience never pass.
pub async fn require_session_or_proxy_token(
    State(deployment): State<DeploymentImpl>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut request = request;

    // Browser branch first: independent of node-runner availability. Resolve, insert, and
    // return BEFORE touching node-runner state (compile-order contract).
    if let Some(ctx) = resolve_browser_session(&deployment.db().pool, request.headers()).await {
        request.extensions_mut().insert(ctx);
        return Ok(next.run(request).await);
    }

    // Non-browser branch: establish this node's own identity, failing closed at each layer.
    let runner = deployment
        .node_runner_context()
        .ok_or(StatusCode::UNAUTHORIZED)?; // outer Option<&NodeRunnerContext>
    let expected_node_id = runner.node_id().await.ok_or(StatusCode::UNAUTHORIZED)?; // inner async Option<Uuid>

    let token = bearer_token(request.headers());
    let validator = deployment.connection_token_validator();
    if proxy_token_is_valid_for_node(validator, token, expected_node_id) {
        Ok(next.run(request).await)
    } else {
        tracing::warn!("rejecting request without a valid browser session or proxy token");
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use secrecy::{ExposeSecret, SecretString};
    use services::services::connection_token::{
        ConnectionTokenClaims, ConnectionTokenValidator, ProxyTokenClaims,
    };
    use uuid::Uuid;

    use super::*;

    fn test_secret() -> SecretString {
        SecretString::from(STANDARD.encode([0x42_u8; 32]))
    }
    fn mint_connection_token(
        secret: &SecretString,
        node_id: Uuid,
        resource: Option<Uuid>,
    ) -> String {
        let now = chrono::Utc::now();
        let claims = ConnectionTokenClaims {
            sub: Uuid::new_v4(),
            node_id,
            assignment_id: Uuid::new_v4(),
            execution_process_id: resource,
            iat: now.timestamp(),
            exp: (now + chrono::Duration::minutes(15)).timestamp(),
            aud: "connection".into(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_base64_secret(secret.expose_secret()).unwrap(),
        )
        .unwrap()
    }
    fn mint_proxy_token(secret: &SecretString, target: Uuid) -> String {
        let now = chrono::Utc::now();
        let claims = ProxyTokenClaims {
            sub: Uuid::new_v4().to_string(),
            node_id: target.to_string(),
            iat: now.timestamp(),
            exp: (now + chrono::Duration::minutes(15)).timestamp(),
            aud: "node_proxy".into(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_base64_secret(secret.expose_secret()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn each_predicate_requires_its_own_audience_node_and_resource_scope() {
        let secret = test_secret();
        let expected_node = uuid::Uuid::new_v4();
        let resource = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        let v = ConnectionTokenValidator::new(secret.clone());
        let conn = mint_connection_token(&secret, expected_node, Some(resource));
        let unscoped = mint_connection_token(&secret, expected_node, None);
        let wrong_node_conn = mint_connection_token(&secret, other, Some(resource));
        let proxy = mint_proxy_token(&secret, expected_node);
        let wrong_node_proxy = mint_proxy_token(&secret, other);

        assert!(connection_token_is_valid_for_resource(
            &v,
            Some(&conn),
            expected_node,
            resource
        ));
        assert!(!connection_token_is_valid_for_resource(
            &v,
            Some(&proxy),
            expected_node,
            resource
        ));
        assert!(!connection_token_is_valid_for_resource(
            &v,
            Some(&unscoped),
            expected_node,
            resource
        ));
        assert!(!connection_token_is_valid_for_resource(
            &v,
            Some(&conn),
            expected_node,
            other
        ));
        assert!(!connection_token_is_valid_for_resource(
            &v,
            Some(&wrong_node_conn),
            expected_node,
            resource
        ));

        assert!(proxy_token_is_valid_for_node(
            &v,
            Some(&proxy),
            expected_node
        ));
        assert!(!proxy_token_is_valid_for_node(
            &v,
            Some(&conn),
            expected_node
        ));
        assert!(!proxy_token_is_valid_for_node(
            &v,
            Some(&wrong_node_proxy),
            expected_node
        ));
        assert!(!connection_token_is_valid_for_resource(
            &v,
            None,
            expected_node,
            resource
        ));
        assert!(!proxy_token_is_valid_for_node(&v, None, expected_node));
    }

    #[test]
    fn bearer_token_accepts_bearer_and_lowercase_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer tok-upper".parse().unwrap(),
        );
        assert_eq!(bearer_token(&headers), Some("tok-upper"));
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "bearer tok-lower".parse().unwrap(),
        );
        assert_eq!(bearer_token(&headers), Some("tok-lower"));
        // RFC 9110: the auth-scheme is case-insensitive.
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "BEARER tok-all-caps".parse().unwrap(),
        );
        assert_eq!(bearer_token(&headers), Some("tok-all-caps"));
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "BeArEr tok-mixed".parse().unwrap(),
        );
        assert_eq!(bearer_token(&headers), Some("tok-mixed"));
    }

    #[test]
    fn bearer_token_rejects_non_bearer_scheme_and_missing_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Basic abc".parse().unwrap(),
        );
        assert_eq!(bearer_token(&headers), None);
        assert_eq!(bearer_token(&HeaderMap::new()), None);
    }

    #[test]
    fn query_token_percent_decodes_and_ignores_other_keys() {
        let with_token: Uri = "/logs?token=a%2Fb&other=1".parse().unwrap();
        assert_eq!(query_token(&with_token).as_deref(), Some("a/b"));
        let no_query: Uri = "/logs".parse().unwrap();
        assert_eq!(query_token(&no_query), None);
        let other_only: Uri = "/logs?foo=bar".parse().unwrap();
        assert_eq!(query_token(&other_only), None);
    }
}
