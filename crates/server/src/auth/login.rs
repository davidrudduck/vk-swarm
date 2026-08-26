//! The ordered browser-login transaction: redeem, identify, pin, persist, mint -- kept out of
//! the route handler so the ordering is reviewable in one screen.

use db::models::browser_auth::{BrowserAuthError, create_session, pin_or_verify_owner};
use deployment::Deployment;
use services::RemoteClientError;
use services::services::oauth_credentials::Credentials;
use utils::api::oauth::HandoffRedeemRequest;
use utils::jwt::extract_expiration;
use uuid::Uuid;

use crate::DeploymentImpl;
use crate::auth::seams::{Clock, OsTokenSource, SystemClock, TokenSource, hash_token};

#[derive(Debug, thiserror::Error)]
pub enum BrowserLoginError {
    #[error("this node is owned by a different hive account")]
    OwnerMismatch,
    /// Static sanitized Display: the wrapped decode failure must never reach a log line or an
    /// HTTP body (SC10). Malformed candidate JWTs are this variant, never `OwnerMismatch` and
    /// never `Remote`.
    #[error("candidate access token is invalid")]
    InvalidToken(#[from] utils::jwt::TokenClaimsError),
    /// A disconnect bumped the browser-auth epoch between claim and commit. Also statically
    /// worded: it names no handoff, no subject and no upstream detail.
    #[error("this sign-in was interrupted by a disconnect; please start again")]
    Disconnected,
    #[error("failed to persist hive credentials")]
    CredentialPersistence(#[source] std::io::Error),
    /// Static sanitized Display: `RemoteClientError::Http` renders `http {status}: {body}` and an
    /// upstream 5xx body can carry reflected sentinels, while the route logs `error = %e`. The
    /// wrapped source stays reachable for programmatic handling; only the Display is static.
    #[error("remote service error")]
    Remote(#[from] RemoteClientError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    /// `remote_client()` is `Err` only when the node has no hive configured at all; unreachable
    /// for a handoff whose initiation already required a configured client. Kept as its own
    /// statically-worded variant so it can never be confused with an upstream failure.
    #[error("hive is not configured on this node")]
    NotConfigured,
}

/// Redeem, identify, pin, persist, mint -- IN THAT ORDER.
///
/// 1. redeem the claimed handoff into CANDIDATE credentials (never saved yet);
/// 2. fetch `ProfileResponse.user_id` with the candidate token via `profile_with_token`;
/// 3. `pin_or_verify_owner` -- first subject pins, same subject passes, different subject is
///    rejected here, BEFORE anything is written: no credential replacement, no owner change and
///    no session revocation (D4/SC6);
/// 4. only then save the daemon credentials; a save failure aborts WITHOUT minting a session;
/// 5. mint the opaque session, storing only its hash.
///
/// Steps 1-3 run with NO epoch guard held: redemption and candidate profile I/O never wait on
/// (and never block) the browser-auth epoch. Immediately before the first credential/session
/// side effect, the shared epoch guard is acquired and `*guard` compared to `epoch_at_claim`
/// (captured under the same mutex at claim time); a mismatch returns
/// `BrowserLoginError::Disconnected` without saving anything. While the matching guard remains
/// held, the refresh guard is acquired too, so an older in-flight token refresh cannot
/// overwrite the accepted candidate after the save.
///
/// A crash after step 3 can leave only the subject pinned; the same owner retries safely.
/// Returns the RAW session token, which the caller puts in exactly one place: the Set-Cookie
/// header. It is never logged and never returned in a body.
pub async fn complete_browser_login(
    deployment: &DeploymentImpl,
    handoff_id: Uuid,
    app_code: String,
    app_verifier: String,
    epoch_at_claim: u64,
) -> Result<String, BrowserLoginError> {
    // 1. Candidate credentials only -- nothing is persisted yet.
    let client = deployment
        .remote_client()
        .map_err(|_| BrowserLoginError::NotConfigured)?;
    let redeem = client
        .handoff_redeem(&HandoffRedeemRequest {
            handoff_id,
            app_code,
            app_verifier,
        })
        .await?;

    // 2. The candidate token must be a decodable JWT with an `exp` claim before it is used or
    //    trusted; failures flow through the sanitized `InvalidToken` variant.
    let expires_at = extract_expiration(&redeem.access_token)?;
    let candidate = Credentials {
        access_token: Some(redeem.access_token.clone()),
        refresh_token: redeem.refresh_token.clone(),
        expires_at: Some(expires_at),
    };

    // 3. Identify the candidate WITH the candidate token -- never the saved daemon credentials
    //    and never the cached profile (both would describe the previous owner).
    let profile = client.profile_with_token(&redeem.access_token).await?;

    // First subject pins, same subject passes, different subject is rejected here with NOTHING
    // written (the db error type is remapped because `BrowserAuthError` is not this crate's).
    pin_or_verify_owner(
        &deployment.db().pool,
        profile.user_id,
        SystemClock.now_millis(),
    )
    .await
    .map_err(|e| match e {
        BrowserAuthError::OwnerMismatch => BrowserLoginError::OwnerMismatch,
        BrowserAuthError::Database(err) => BrowserLoginError::Database(err),
    })?;

    // 4+5. Fenced commit: the epoch re-check happens immediately before the first
    //      credential/session side effect, and both guards stay held across save + mint + the
    //      synchronous remote-sync installation and node-cache start.
    let epoch_guard = deployment.browser_auth_epoch().lock().await;
    if *epoch_guard != epoch_at_claim {
        return Err(BrowserLoginError::Disconnected);
    }
    let refresh_guard = deployment.auth_context().refresh_guard().await;
    deployment
        .auth_context()
        .save_credentials(&candidate)
        .await
        .map_err(BrowserLoginError::CredentialPersistence)?;
    let raw_token = OsTokenSource.generate_token();
    create_session(
        &deployment.db().pool,
        Uuid::new_v4(),
        &hash_token(&raw_token),
        profile.user_id,
        SystemClock.now_millis(),
    )
    .await?;
    if let Some(config) = deployment.share_config() {
        deployment.install_remote_sync(config.clone()).await;
    }
    // Node-cache start stays inside the epoch fence so a concurrent disconnect that already
    // shut the previous handle cannot race a replacement spawn after this guard drops.
    deployment.start_node_cache_sync().await;
    drop(refresh_guard);
    drop(epoch_guard);
    Ok(raw_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `Remote` variant's Display must stay static: `RemoteClientError::Http` renders
    /// `http {status}: {body}`, and the route logs `error = %e`, so a transparent Display
    /// would leak an upstream body (which can carry reflected sentinels) into the logs.
    #[test]
    fn remote_variant_display_is_sanitized() {
        let inner = RemoteClientError::Http {
            status: 500,
            body: "SENTINEL-ACCESS-8f31c0d2".to_string(),
        };
        let wrapped = BrowserLoginError::Remote(inner);
        assert_eq!(wrapped.to_string(), "remote service error");
        assert!(
            !wrapped.to_string().contains("SENTINEL"),
            "upstream body leaked through Display: {}",
            wrapped
        );
    }
}
