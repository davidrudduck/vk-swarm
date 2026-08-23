use axum::{Router, extract::State, http::HeaderMap, response::Json as ResponseJson, routing::get};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::DeploymentImpl;

/// The only thing an unauthorized browser may learn: whether THIS browser is authorized and
/// whether OAuth can currently be started. Deliberately carries no config, environment,
/// executor, node or profile data (D8) -- the login shell needs nothing else.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BrowserAuthState {
    pub authorized: bool,
    pub oauth_available: bool,
}

pub fn public_router() -> Router<DeploymentImpl> {
    Router::new().route("/auth/state", get(auth_state))
}

async fn auth_state(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
) -> ResponseJson<ApiResponse<BrowserAuthState>> {
    let authorized = crate::auth::session::resolve_browser_session(&deployment.db().pool, &headers)
        .await
        .is_some();
    // `remote_client()` is Err only when the node has no hive configured; a hive OUTAGE does not
    // change this flag, and neither flag depends on hive reachability (SC9).
    let oauth_available = deployment.remote_client().is_ok();
    ResponseJson(ApiResponse::success(BrowserAuthState {
        authorized,
        oauth_available,
    }))
}
