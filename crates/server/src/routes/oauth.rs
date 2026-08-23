use axum::{
    Router,
    extract::{Json, Query, State},
    http::{Response, StatusCode},
    response::{IntoResponse, Json as ResponseJson},
    routing::{get, post},
};
use db::models::browser_auth::{
    claim_handoff, create_handoff, invalidate_pending_handoffs, revoke_all_sessions, revoke_session,
};
use deployment::Deployment;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utils::{
    api::oauth::{HandoffInitRequest, LoginStatus, StatusResponse},
    response::ApiResponse,
};
use uuid::Uuid;

use crate::auth::cookies::{
    BINDING_COOKIE, SESSION_COOKIE, binding_set_cookie, read_cookie, session_clear_cookie,
    session_set_cookie,
};
use crate::auth::login::{BrowserLoginError, complete_browser_login};
use crate::auth::seams::{Clock, OsTokenSource, SystemClock, TokenSource, hash_token};
use crate::{DeploymentImpl, error::ApiError};

/// PUBLIC: a browser must be able to start and finish OAuth before it has any session.
pub fn public_router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/auth/handoff/init", post(handoff_init))
        .route("/auth/handoff/complete", get(handoff_complete))
}

/// PROTECTED: `/auth/logout` is the explicit daemon/Hive DISCONNECT action (it stops sync and
/// removes daemon credentials) and `/auth/status` returns the hive profile -- neither may be
/// reachable without a browser session. The browser-scoped logout is added by task 012 as a
/// separately named route on this same router.
pub fn protected_router() -> Router<DeploymentImpl> {
    Router::new()
        // Browser-scoped: revokes ONLY the presenting browser (SC7).
        .route("/auth/browser/logout", post(browser_logout))
        // Daemon/Hive DISCONNECT, kept under its existing name and semantics: revoke every
        // session, stop sync, remove daemon credentials (SC8).
        //
        // Keeping the name is a REVERSIBLE backward-compatibility choice, not a hard constraint:
        // `frontend/src/lib/api/oauth.ts` already exposes `oauthApi.logout()` bound to
        // POST /api/auth/logout, and that endpoint already means "disconnect the daemon" (stop
        // sync, clear credentials). Adding the browser-scoped action under a NEW path leaves
        // every existing caller correct, whereas renaming would silently change what an
        // unmigrated caller does. D5 requires only that the two operations be separately NAMED.
        // If a later workstream prefers /auth/disconnect, it is a one-line route rename plus the
        // caller update -- fully reversible.
        .route("/auth/logout", post(logout))
        .route("/auth/status", get(status))
}

#[derive(Debug, Deserialize)]
struct HandoffInitPayload {
    provider: String,
    return_to: String,
}

#[derive(Debug, Serialize)]
struct HandoffInitResponseBody {
    handoff_id: Uuid,
    authorize_url: String,
}

async fn handoff_init(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<HandoffInitPayload>,
) -> Result<axum::response::Response, ApiError> {
    let client = deployment.remote_client()?;

    let app_verifier = generate_secret();
    let app_challenge = hash_sha256_hex(&app_verifier);

    let request = HandoffInitRequest {
        provider: payload.provider.clone(),
        return_to: payload.return_to.clone(),
        app_challenge,
    };
    let response = client.handoff_init(&request).await?;

    // A fresh browser-held secret per initiation. Only its hash is persisted; the raw value
    // exists solely in this Set-Cookie header and the presenting browser, which is what makes a
    // copied callback URL useless in another browser (D3/SC3).
    let binding_token = OsTokenSource.generate_token();
    let binding_hash = hash_token(&binding_token);
    let now_millis = SystemClock.now_millis();

    // The DB insert is the initiation linearization point. Hive I/O is deliberately outside this
    // short guard. If disconnect linearizes first, this is a legitimate fresh post-disconnect
    // handoff; if this insert linearizes first, disconnect durably invalidates it.
    let _epoch_guard = deployment.browser_auth_epoch().lock().await;
    create_handoff(
        &deployment.db().pool,
        response.handoff_id,
        &payload.provider,
        &app_verifier,
        &binding_hash,
        now_millis,
    )
    .await
    .map_err(ApiError::Database)?;

    let body = HandoffInitResponseBody {
        handoff_id: response.handoff_id,
        authorize_url: response.authorize_url,
    };

    Ok((
        [(
            axum::http::header::SET_COOKIE,
            binding_set_cookie(&binding_token),
        )],
        ResponseJson(ApiResponse::<HandoffInitResponseBody>::success(body)),
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
struct HandoffCompleteQuery {
    handoff_id: Uuid,
    #[serde(default)]
    app_code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

async fn handoff_complete(
    State(deployment): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Query(query): Query<HandoffCompleteQuery>,
) -> Result<Response<String>, ApiError> {
    if let Some(error) = query.error {
        return Ok(simple_html_response(
            StatusCode::BAD_REQUEST,
            format!("OAuth authorization failed: {error}"),
        ));
    }

    let Some(app_code) = query.app_code.clone() else {
        return Ok(simple_html_response(
            StatusCode::BAD_REQUEST,
            "Missing app_code in callback".to_string(),
        ));
    };

    // Claim BEFORE any hive I/O. One conditional UPDATE decides the single consumer: a
    // wrong-browser, expired or replayed attempt matches no row and therefore consumes nothing,
    // leaving a rightful pending handoff exactly as it was (SC3/SC4).
    let binding_hash = match read_cookie(&headers, BINDING_COOKIE) {
        Some(raw) => hash_token(&raw),
        None => {
            tracing::warn!(handoff_id = %query.handoff_id, "callback without a binding cookie");
            return Ok(simple_html_response(
                StatusCode::BAD_REQUEST,
                "This browser did not start this sign-in. Start again from the app.".to_string(),
            ));
        }
    };

    // Claim and epoch capture are one short linearization section. Disconnect cannot fit between
    // them and make a stale callback appear current at commit time. No Hive I/O runs under it.
    let epoch_guard = deployment.browser_auth_epoch().lock().await;
    let epoch_at_claim = *epoch_guard;
    let claimed = claim_handoff(
        &deployment.db().pool,
        query.handoff_id,
        &binding_hash,
        SystemClock.now_millis(),
    )
    .await
    .map_err(ApiError::Database)?;
    drop(epoch_guard);

    let Some(handoff) = claimed else {
        // Deliberately ONE message for unknown / wrong-browser / expired / already-claimed: the
        // distinction is not the browser's business, and a claimed row is terminal either way --
        // recovery is a fresh initiation, never a re-claim.
        tracing::warn!(handoff_id = %query.handoff_id, "handoff not claimable");
        return Ok(simple_html_response(
            StatusCode::BAD_REQUEST,
            "OAuth handoff not found, expired, or already completed".to_string(),
        ));
    };
    let (provider, app_verifier) = (handoff.provider, handoff.app_verifier);

    let session_token = match complete_browser_login(
        &deployment,
        query.handoff_id,
        app_code,
        app_verifier,
        epoch_at_claim,
    )
    .await
    {
        Ok(token) => token,
        Err(BrowserLoginError::OwnerMismatch) => {
            // Rejection is side-effect free: no credentials saved, owner unchanged, no session
            // revoked. Owner reset is deliberately out of scope.
            tracing::warn!(handoff_id = %query.handoff_id, "rejected a different hive subject");
            return Ok(simple_html_response(
                StatusCode::BAD_REQUEST,
                "This node is already owned by a different account.".to_string(),
            ));
        }
        Err(e) => {
            // `e` is Display-formatted deliberately: Debug on a redemption error can carry the
            // candidate token (SC10).
            tracing::error!(handoff_id = %query.handoff_id, error = %e, "browser login failed");
            return Ok(simple_html_response(
                StatusCode::BAD_REQUEST,
                "Sign-in could not be completed. Please start again.".to_string(),
            ));
        }
    };

    // Start node cache sync to fetch all nodes/projects from other nodes in the org
    deployment.start_node_cache_sync().await;

    let mut response = close_window_response(format!(
        "Signed in with {provider}. You can return to the app."
    ));
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&session_set_cookie(&session_token))
            .expect("session cookie is ascii"),
    );
    Ok(response)
}

/// Revoke ONLY the presenting browser's session and expire its cookie.
///
/// Does not stop sync, does not touch daemon Hive credentials, does not touch the pinned owner,
/// and does not affect any other browser. Idempotent: revoking an already-revoked session is a
/// success, because the operator's intent (this browser is signed out) is satisfied either way.
async fn browser_logout(
    State(deployment): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    if let Some(raw) = read_cookie(&headers, SESSION_COOKIE) {
        revoke_session(
            &deployment.db().pool,
            &hash_token(&raw),
            SystemClock.now_millis(),
        )
        .await
        .map_err(ApiError::Database)?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(axum::http::header::SET_COOKIE, session_clear_cookie())],
    )
        .into_response())
}

/// Explicit Hive DISCONNECT (D5/SC8). Order matters and is fixed by O8: SQLite session
/// revocation and file/Keychain credential deletion cannot share a transaction, so revoke every
/// browser session FIRST -- if credential removal then fails, the node is at worst
/// over-locked-out rather than leaving live browsers on a node whose credentials are gone.
///
/// The pinned owner is deliberately RETAINED: a disconnected trusted-LAN node must not become
/// claimable by a different Hive subject through ordinary OAuth (D4).
async fn logout(State(deployment): State<DeploymentImpl>) -> Result<StatusCode, ApiError> {
    let mut epoch_guard = deployment.browser_auth_epoch().lock().await;
    *epoch_guard = epoch_guard.wrapping_add(1);
    let invalidated = invalidate_pending_handoffs(&deployment.db().pool)
        .await
        .map_err(ApiError::Database)?;
    let revoked = revoke_all_sessions(&deployment.db().pool, SystemClock.now_millis())
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(
        invalidated,
        revoked,
        "invalidated pending logins and revoked all browser sessions for hive disconnect"
    );

    // Stop remote sync if running. Take the handle out of its slot and drop the slot guard
    // BEFORE awaiting shutdown(): the fenced browser-login commit holds `browser_auth_epoch` +
    // `refresh_guard` across `install_remote_sync`, which locks this same slot — holding the
    // slot across the shutdown join would complete a three-party deadlock cycle when the
    // RemoteSync task is itself blocked on `refresh_guard`.
    let handle = { deployment.share_sync_handle().lock().await.take() };
    if let Some(handle) = handle {
        tracing::info!("Stopping remote sync due to logout");
        handle.shutdown().await;
    }

    // Stop every Hive synchronization task if running. Task 006 makes node-cache work owned and
    // awaitable rather than detached.
    deployment.shutdown_node_cache_sync().await;

    let auth_context = deployment.auth_context();

    if let Ok(client) = deployment.remote_client() {
        let _ = client.logout().await;
    }

    // Serialize only credential clearing against token refresh. Do NOT take this guard before
    // client.logout(): that call may itself refresh and tokio Mutex is not re-entrant.
    let refresh_guard = deployment.auth_context().refresh_guard().await;
    auth_context.clear_credentials().await.map_err(|e| {
        tracing::error!(?e, "failed to clear credentials");
        ApiError::Io(e)
    })?;
    drop(refresh_guard);
    auth_context.clear_profile().await;

    Ok(StatusCode::NO_CONTENT)
}

async fn status(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<StatusResponse>>, ApiError> {
    match deployment.get_login_status().await {
        LoginStatus::LoggedOut => Ok(ResponseJson(ApiResponse::success(StatusResponse {
            logged_in: false,
            profile: None,
            degraded: None,
        }))),
        LoginStatus::LoggedIn { profile } => {
            Ok(ResponseJson(ApiResponse::success(StatusResponse {
                logged_in: true,
                profile: Some(profile),
                degraded: None,
            })))
        }
    }
}

fn generate_secret() -> String {
    let mut rng = rand::rng();
    (0..64)
        .map(|_| {
            let idx = rng.random_range(0..62);
            const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
            CHARS[idx] as char
        })
        .collect()
}

fn hash_sha256_hex(input: &str) -> String {
    let mut output = String::with_capacity(64);
    let digest = Sha256::digest(input.as_bytes());
    for byte in digest {
        // Scoped import: `Write` trait name conflicts with std::io::Write
        use std::fmt::Write;
        let _ = write!(output, "{:02x}", byte);
    }
    output
}

fn simple_html_response(status: StatusCode, message: String) -> Response<String> {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>OAuth</title></head>\
         <body style=\"font-family: sans-serif; margin: 3rem;\"><h1>{}</h1></body></html>",
        message
    );
    Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .body(body)
        .unwrap()
}

fn close_window_response(message: String) -> Response<String> {
    let body = format!(
        "<!doctype html>\
         <html>\
           <head>\
             <meta charset=\"utf-8\">\
             <title>Authentication Complete</title>\
             <script>\
               window.addEventListener('load', () => {{\
                 try {{ window.close(); }} catch (err) {{}}\
                 setTimeout(() => {{ window.close(); }}, 150);\
               }});\
             </script>\
             <style>\
               body {{ font-family: sans-serif; margin: 3rem; color: #1f2933; }}\
             </style>\
           </head>\
           <body>\
             <h1>{}</h1>\
             <p>If this window does not close automatically, you may close it manually.</p>\
           </body>\
         </html>",
        message
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(body)
        .unwrap()
}
