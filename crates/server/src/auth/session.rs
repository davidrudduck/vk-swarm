//! Browser-session resolution and the rejecting layer for the protected router.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use deployment::Deployment;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::DeploymentImpl;
use crate::auth::cookies::{SESSION_COOKIE, read_cookie};
use crate::auth::seams::hash_token;

/// What an authorized request carries downstream.
#[derive(Debug, Clone)]
pub struct BrowserSessionCtx {
    pub session_id: Uuid,
    pub hive_user_id: Uuid,
}

/// NON-rejecting resolution: Some when the presented cookie hashes to a live session.
///
/// Evaluates ONLY the stored token hash and revocation state. It never consults Hive, never
/// checks elapsed time, and therefore cannot be broken by a Hive outage (D6/SC9). This is the
/// function `GET /api/auth/state` uses -- that route must answer 200 with `authorized:false`
/// for a clean browser, so it must NOT sit behind a rejecting layer.
pub async fn resolve_browser_session(
    pool: &SqlitePool,
    headers: &HeaderMap,
) -> Option<BrowserSessionCtx> {
    let raw = read_cookie(headers, SESSION_COOKIE)?;
    let token_hash = hash_token(&raw);
    match db::models::browser_auth::authenticate_session(pool, &token_hash).await {
        Ok(Some(session)) => Some(BrowserSessionCtx {
            session_id: session.id,
            hive_user_id: session.hive_user_id,
        }),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(error = ?e, "browser session lookup failed; failing closed");
            None
        }
    }
}

/// REJECTING layer for the protected router. 401 when no live session is presented, otherwise
/// inserts `Extension<BrowserSessionCtx>` and calls `next`.
///
/// Runs BEFORE any route-specific extractor, resource lookup or protocol upgrade, because it is
/// layered on the whole protected subtree rather than on individual handlers (D1).
pub async fn require_browser_session(
    State(deployment): State<DeploymentImpl>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut request = request;
    // Standalone node (no hive configured): there is no Hive login to bind a browser to,
    // so the gate is fully open. No BrowserSessionCtx exists in this mode; downstream
    // handlers already treat it as optional.
    if deployment.is_standalone() {
        return Ok(next.run(request).await);
    }
    match resolve_browser_session(&deployment.db().pool, request.headers()).await {
        Some(ctx) => {
            request.extensions_mut().insert(ctx);
            Ok(next.run(request).await)
        }
        None => {
            tracing::warn!("rejecting request without a live browser session");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolver_hashes_the_presented_cookie_and_honours_revocation() {
        let (pool, _t) = db::test_utils::create_test_pool().await;
        let raw = "raw-session-token";
        let owner = uuid::Uuid::new_v4();
        db::models::browser_auth::create_session(
            &pool,
            uuid::Uuid::new_v4(),
            &crate::auth::seams::hash_token(raw),
            owner,
            1,
        )
        .await
        .unwrap();

        let mut h = axum::http::HeaderMap::new();
        h.insert(
            axum::http::header::COOKIE,
            format!("{}={raw}", crate::auth::cookies::SESSION_COOKIE)
                .parse()
                .unwrap(),
        );
        let ctx = resolve_browser_session(&pool, &h)
            .await
            .expect("live session must resolve");
        assert_eq!(ctx.hive_user_id, owner);

        // Presenting the STORED HASH must NOT authorize: the server hashes what it receives.
        let mut hh = axum::http::HeaderMap::new();
        hh.insert(
            axum::http::header::COOKIE,
            format!(
                "{}={}",
                crate::auth::cookies::SESSION_COOKIE,
                crate::auth::seams::hash_token(raw)
            )
            .parse()
            .unwrap(),
        );
        assert!(resolve_browser_session(&pool, &hh).await.is_none());

        assert!(
            resolve_browser_session(&pool, &axum::http::HeaderMap::new())
                .await
                .is_none()
        );
        db::models::browser_auth::revoke_session(&pool, &crate::auth::seams::hash_token(raw), 2)
            .await
            .unwrap();
        assert!(resolve_browser_session(&pool, &h).await.is_none());
    }

    #[tokio::test]
    async fn resolver_fails_closed_when_the_database_errors() {
        // A DB failure must surface as None (fail closed), never a panic and never a
        // fabricated session. Discriminates unwrap/expect mutants on the query result
        // and any fallback that invents a BrowserSessionCtx on error.
        let (pool, _t) = db::test_utils::create_test_pool().await;
        let raw = "raw-session-token";
        db::models::browser_auth::create_session(
            &pool,
            uuid::Uuid::new_v4(),
            &crate::auth::seams::hash_token(raw),
            uuid::Uuid::new_v4(),
            1,
        )
        .await
        .unwrap();

        let mut h = axum::http::HeaderMap::new();
        h.insert(
            axum::http::header::COOKIE,
            format!("{}={raw}", crate::auth::cookies::SESSION_COOKIE)
                .parse()
                .unwrap(),
        );

        pool.close().await;
        assert!(resolve_browser_session(&pool, &h).await.is_none());
    }
}
