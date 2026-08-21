---
id: "011"
phase: 2
title: "Validate the candidate profile and owner before saving credentials, then mint the browser session"
status: ready
depends_on: ["003","005","010"]
parallel: false
conflicts_with: ["002","007","008","009","010","012"]
files:
  - "crates/services/src/services/remote_client.rs"
  - "crates/server/src/auth/login.rs"
  - "crates/server/src/auth/mod.rs"
  - "crates/server/src/routes/oauth.rs"
  - "crates/server/tests/browser_oauth.rs"
siblings: ["crates/services/src/services/oauth_credentials.rs","crates/server/tests/events.rs","crates/server/tests/harness_smoke.rs","crates/server/tests/mcp_context_test.rs"]
irreversible: false
scope_test: "crates/server/tests/browser_oauth.rs"
allowed_change: mixed
covers_criteria: ["SC5","SC6"]
covers_tests: ["TS2"]
---
## Failing test (write first)
Append to `crates/server/tests/browser_oauth.rs`:

```rust
#[tokio::test]
#[serial_test::serial]
async fn successful_login_mints_a_hash_only_persistent_session_cookie() {
    let h = common::HiveHarness::configured().await;
    let owner = uuid::Uuid::new_v4();
    let id = h.mock_hive_oauth("code-1", "acc", "ref", owner).await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    let done = h.get_with(&cb, &mut a).await;
    assert_eq!(done.status, 200, "body: {}", done.body);

    let line = done.set_cookie.iter().find(|c| c.starts_with("vks_browser_session="))
        .expect("no session cookie");
    assert!(line.contains("HttpOnly") && line.contains("SameSite=Lax")
        && line.contains("Path=/") && line.contains("Max-Age=157680000"), "{line}");
    assert!(!line.contains("Secure"), "{line}");

    let raw = a.get("vks_browser_session").unwrap().to_string();
    let stored: Vec<String> = sqlx::query_scalar("SELECT token_hash FROM browser_sessions")
        .fetch_all(h.pool()).await.unwrap();
    assert_eq!(stored, vec![server::auth::seams::hash_token(&raw)]);
    assert!(!stored.contains(&raw), "the raw session token must never be stored");
    assert!(!done.body.contains(&raw), "session token leaked into the response body");

    // The authorized browser now reaches protected data; a clean browser still does not.
    let info = h.get_with("/api/info", &mut a).await;
    assert_eq!(info.status, 200, "body: {}", info.body);
    let mut b = common::CookieJar::fresh();
    assert_eq!(h.get_with("/api/info", &mut b).await.status, 401);
    let state_b = h.get_with("/api/auth/state", &mut b).await;
    assert!(state_b.body.contains("\"authorized\":false"), "{}", state_b.body);
}

#[tokio::test]
#[serial_test::serial]
async fn the_same_owner_may_authorize_a_second_browser() {
    let h = common::HiveHarness::configured().await;
    let owner = uuid::Uuid::new_v4();
    let id1 = h.mock_hive_oauth("code-1", "acc", "ref", owner).await;
    let mut a = common::CookieJar::new();
    let cb1 = start_login(&h, &mut a, id1).await;
    assert_eq!(h.get_with(&cb1, &mut a).await.status, 200);

    let id2 = h.mock_hive_oauth("code-2", "acc2", "ref2", owner).await;
    let mut b = common::CookieJar::fresh();
    let cb2 = format!("/api/auth/handoff/complete?handoff_id={id2}&app_code=code-2");
    h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut b).await;
    assert_eq!(h.get_with(&cb2, &mut b).await.status, 200);

    assert_eq!(h.get_with("/api/info", &mut a).await.status, 200, "first session survived");
    assert_eq!(h.get_with("/api/info", &mut b).await.status, 200);
}

#[tokio::test]
#[serial_test::serial]
async fn a_different_subject_is_rejected_without_replacing_credentials_or_sessions() {
    let h = common::HiveHarness::configured().await;
    let owner = uuid::Uuid::new_v4();
    let id1 = h.mock_hive_oauth("code-1", "acc", "ref", owner).await;
    let mut a = common::CookieJar::new();
    let cb1 = start_login(&h, &mut a, id1).await;
    assert_eq!(h.get_with(&cb1, &mut a).await.status, 200);
    let creds_before = std::fs::read_to_string(h.credentials_path()).unwrap();

    let intruder = uuid::Uuid::new_v4();
    let id2 = h.mock_hive_oauth("code-2", "intruder-access", "intruder-refresh", intruder).await;
    let mut c = common::CookieJar::fresh();
    h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut c).await;
    let res = h.get_with(
        &format!("/api/auth/handoff/complete?handoff_id={id2}&app_code=code-2"), &mut c).await;
    assert_eq!(res.status, 400, "body: {}", res.body);
    assert!(res.set_cookie.iter().all(|l| !l.starts_with("vks_browser_session=")));

    // Owner unchanged, daemon credentials untouched, existing session still authorized.
    let pinned: Vec<u8> = sqlx::query_scalar("SELECT hive_user_id FROM node_owner")
        .fetch_one(h.pool()).await.unwrap();
    assert_eq!(uuid::Uuid::from_slice(&pinned).unwrap(), owner);
    assert_eq!(std::fs::read_to_string(h.credentials_path()).unwrap(), creds_before,
        "candidate credentials must never be saved on a rejected subject");
    assert_eq!(h.get_with("/api/info", &mut a).await.status, 200,
        "a rejected login must not revoke existing sessions");
}
```

Append an invalid-candidate-token test: configure redeem to return a malformed JWT label fixture, complete the browser callback, and assert generic HTTP 400 with no upstream body/token text, no owner/session/credential write, and no `OwnerMismatch` classification. A missing/invalid `exp` uses the same test shape.



## Change
**File:** `crates/services/src/services/remote_client.rs`
**Anchor:** the `profile()` method at L530-533.
**Before:**
```rust
    /// Fetches user profile.
    pub async fn profile(&self) -> Result<ProfileResponse, RemoteClientError> {
        self.get_authed("/v1/profile").await
    }
```
**After:**
```rust
    /// Fetches user profile.
    pub async fn profile(&self) -> Result<ProfileResponse, RemoteClientError> {
        self.get_authed("/v1/profile").await
    }

    /// Fetches the profile using an EXPLICIT candidate access token, WITHOUT reading or writing
    /// the stored daemon credentials.
    ///
    /// This is what lets the node learn a candidate's identity BEFORE deciding whether to accept
    /// it as the owner: `profile()` would go through AuthMode::OAuth and use whatever is already
    /// saved. AuthMode::ApiKey passes the supplied token straight to `bearer_auth` (see
    /// `require_token`), which is exactly the semantics needed here.
    pub async fn profile_with_token(&self, access_token: &str)
        -> Result<ProfileResponse, RemoteClientError> {
        Self::new_with_api_key(self.base.as_str(), access_token.to_string())?
            .profile()
            .await
    }
```

**File:** `crates/server/src/auth/mod.rs`
**Before:** `pub mod cookies;
pub mod node_token;
pub mod seams;
pub mod session;`
**After:** `pub mod cookies;
pub mod login;
pub mod node_token;
pub mod seams;
pub mod session;`

**File:** `crates/server/src/auth/login.rs` — create. The ordered login transaction, kept out of the route handler so the ordering is reviewable in one screen:
```rust
#[derive(Debug, thiserror::Error)]
pub enum BrowserLoginError {
    #[error("this node is owned by a different hive account")]
    OwnerMismatch,
    #[error(transparent)]
    Remote(#[from] RemoteClientError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("failed to persist hive credentials")]
    CredentialPersistence(#[source] std::io::Error),
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
/// A crash after step 3 can leave only the subject pinned; the same owner retries safely.
/// Returns the RAW session token, which the caller puts in exactly one place: the Set-Cookie
/// header. It is never logged and never returned in a body.
pub async fn complete_browser_login(
    deployment: &DeploymentImpl,
    handoff_id: Uuid,
    app_code: String,
    app_verifier: String,
) -> Result<String, BrowserLoginError>;
```

**File:** `crates/server/src/routes/oauth.rs`
**Anchor:** in `handoff_complete`, everything from `let client = deployment.remote_client()?;` through the `deployment.auth_context().save_credentials(&credentials)` block (L108-135).
**Before:** redeem -> `extract_expiration` -> build `Credentials` -> `save_credentials` -> `let _ = deployment.get_login_status().await;`
**After:**
```rust
    let session_token = match complete_browser_login(
        &deployment, query.handoff_id, app_code, app_verifier).await
    {
        Ok(token) => token,
        Err(BrowserLoginError::OwnerMismatch) => {
            // Rejection is side-effect free: no credentials saved, owner unchanged, no session
            // revoked. Owner reset is deliberately out of scope.
            tracing::warn!(handoff_id = %query.handoff_id, "rejected a different hive subject");
            return Ok(simple_html_response(StatusCode::BAD_REQUEST,
                "This node is already owned by a different account.".to_string()));
        }
        Err(e) => {
            // `e` is Display-formatted deliberately: Debug on a redemption error can carry the
            // candidate token (SC10).
            tracing::error!(handoff_id = %query.handoff_id, error = %e, "browser login failed");
            return Ok(simple_html_response(StatusCode::BAD_REQUEST,
                "Sign-in could not be completed. Please start again.".to_string()));
        }
    };
```
The sync-spawn and node-cache-sync blocks that follow stay exactly as they are. The final response becomes the same `close_window_response` carrying one added header:
```rust
    let mut response = close_window_response(format!(
        "Signed in with {provider}. You can return to the app."));
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&session_set_cookie(&session_token))
            .expect("session cookie is ascii"),
    );
    Ok(response)
```

**Sibling alignment (rubric 9).** Read `crates/services/src/services/oauth_credentials.rs` before writing step 4: `save_credentials` is the existing persistence contract (and its error type) — reuse it verbatim rather than writing to the credentials file directly.

**SC5 note.** This task proves the cookie's opacity, persistence attributes and hash-only storage. The remaining SC5 clause — survival across a planned idle node restart — is proven by task 015's TS4 suite against the same migrated SQLite/assets directory.

**Symbol grounding:** This task introduces `profile_with_token()` on `RemoteClient` in `crates/services`, and `complete_browser_login()` plus the `BrowserLoginError` type in the new `crates/server/src/auth/login.rs`. It calls `session_set_cookie()` (task 007), `pin_or_verify_owner()` (task 003), `create_session()` (task 005) and `hash_token()` (task 002). `profile()` and `new_with_api_key()` are pre-existing `RemoteClient` methods.

**Candidate expiration error contract.** In `BrowserLoginError` add exactly:
```rust
#[error("candidate access token is invalid")]
InvalidToken(#[from] utils::jwt::TokenClaimsError),
```
The static Display is sanitized while retaining the typed source. Candidate expiration extraction uses `utils::jwt::extract_expiration(&candidate.access_token)?`; decode/missing/invalid-exp failures flow through `InvalidToken` and the existing generic browser-login 400 mapping. Never map them to `OwnerMismatch`, never include the candidate token, a decoded claim, or an upstream response body in the HTTP body or logs.



## Allowed moves
[
  "Add exactly one method to remote_client.rs; change nothing else in crates/services.",
  "Create auth/login.rs and add one `pub mod login;` line.",
  "Replace only the redeem/save block inside handoff_complete and add the Set-Cookie to the success response.",
  "Append the three tests to crates/server/tests/browser_oauth.rs."
]


## STOP triggers
[
  "Saving credentials before the profile fetch or before pin_or_verify_owner — that is exactly the ordering defect this task exists to fix.",
  "Calling `deployment.get_login_status()` (or any cached-profile path) to identify the candidate — it uses the SAVED credentials, so it cannot identify a candidate before saving.",
  "Revoking any session, or changing node_owner, on the OwnerMismatch path.",
  "Minting the session before save_credentials succeeds.",
  "Any `?e` / Debug formatting of a redemption or credential error reaching a log line or an HTTP body — Debug can carry the candidate tokens.",
  "Returning the session token in a JSON body, a redirect Location, or a query parameter.",
  "Adding an owner-reset or owner-replacement operation — out of scope; and any such op would owe a same-transaction revoke-all.",
  "Mapping TokenClaimsError to OwnerMismatch or RemoteClientError — malformed candidate JWTs are BrowserLoginError::InvalidToken.",
  "Displaying the wrapped TokenClaimsError, token, claims or upstream body to the browser; InvalidToken has a static sanitized Display and existing generic 400 handling."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_oauth" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 011` exits 0.
2. `cargo test -p server --test browser_oauth` — 9 tests green; `cargo test -p services` still green.
3. TS2 walk-through recorded in the ledger: name the test covering each TS2 clause (public/protected routing -> 008's suite; browser-A isolation, callback copying/replay -> 010's suite; cookie attributes, same-owner and different-owner redemption -> this suite; browser logout and explicit disconnect -> 012's suite).
4. SC5 restart clause: note that it is proven by task 015 (TS4), not here.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 011` exits 0
