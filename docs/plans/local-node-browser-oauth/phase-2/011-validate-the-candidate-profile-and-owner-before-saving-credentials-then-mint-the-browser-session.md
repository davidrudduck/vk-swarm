---
id: "011"
phase: 2
title: "Validate the candidate profile and owner before saving credentials, then mint the browser session"
status: passed
depends_on: ["003","005","010"]
parallel: false
conflicts_with: ["002","006","007","008","009","010","012","018","022"]
files:
  - "crates/services/src/services/remote_client.rs"
  - "crates/server/src/auth/login.rs"
  - "crates/server/src/auth/mod.rs"
  - "crates/server/src/routes/oauth.rs"
  - "crates/server/tests/browser_oauth.rs"
  - "crates/server/tests/common/mod.rs"
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
    epoch_at_claim: u64,
) -> Result<String, BrowserLoginError>;
```

**File:** `crates/server/src/routes/oauth.rs`
**Anchor:** in `handoff_complete`, everything from `let client = deployment.remote_client()?;` through the `deployment.auth_context().save_credentials(&credentials)` block (L108-135).
**Before:** redeem -> `extract_expiration` -> build `Credentials` -> `save_credentials` -> `let _ = deployment.get_login_status().await;`
**After:**
```rust
    let session_token = match complete_browser_login(
        &deployment, query.handoff_id, app_code, app_verifier, epoch_at_claim).await
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
Inside `complete_browser_login`, redemption, candidate profile fetch and owner verification happen
without the epoch guard. Immediately before the first credential/session side effect, acquire the
shared epoch guard and compare `*guard` to `epoch_at_claim`. A mismatch returns a new sanitized
`BrowserLoginError::Disconnected` without saving credentials or creating a session. While the
matching guard remains held: acquire `auth_context.refresh_guard()`, save credentials, create the
hash-only session, and call `deployment.install_remote_sync(config).await`; then release both
guards. This commit section is the only login path allowed to save credentials or mint a session.
The refresh guard prevents an older in-flight refresh from overwriting the accepted candidate.

Remove the later detached `spawn_remote_sync` call from `handoff_complete`; synchronous installation
already happened inside the fenced commit. The node-cache-sync block stays unchanged. The final
response becomes the same `close_window_response` carrying one added header:
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



## Panel-strengthened corrections (round-1 remediation)

**Sanitized `Remote` Display (SC10; task 018's sentinel obligation is fixed in THIS owning
task).** `RemoteClientError::Http` Displays `http {status}: {body}` — an upstream 5xx body can
carry reflected sentinels, and the route logs `error = %e`. The `Remote` variant must therefore
NOT be `#[error(transparent)]`. Use exactly:
```rust
#[error("remote service error")]
Remote(#[from] RemoteClientError),
```
(`#[from]` is retained so `?` keeps working; only the Display becomes static.) A same-file unit
test constructs `RemoteClientError::Http { status: 500, body: "SENTINEL-ACCESS-8f31c0d2".into() }`,
wraps it, and asserts `to_string()` equals `"remote service error"` (no sentinel substring).

**Logout slot-guard release (deadlock cycle introduced by this task's fenced commit).** The
fenced commit holds `browser_auth_epoch` + `refresh_guard` across `install_remote_sync`, which
locks the share-sync slot. The existing `logout` holds that slot across `handle.shutdown().await`
(join), and the RemoteSync task can be blocked on `refresh_guard` inside an in-flight authed
call — a reachable three-party cycle. Fix in `logout` (take-before-await, the pattern used
everywhere else):
```rust
let handle = { deployment.share_sync_handle().lock().await.take() };
if let Some(handle) = handle {
    tracing::info!("Stopping remote sync due to logout");
    handle.shutdown().await;
}
```
Task 012 owns the full disconnect semantics; this change only removes the cycle.

**New harness helper (additive, mirrors `mock_hive_delayed`).** In
`crates/server/tests/common/mod.rs`:
```rust
/// Priority-1 override that signals on arrival, then answers `body` after `delay_ms`.
pub async fn mock_hive_delayed_json(
    &self,
    method: &str,
    path: &str,
    delay_ms: u64,
    body: serde_json::Value,
) -> tokio::sync::oneshot::Receiver<()>
```
(same signal-then-delay responder shape as `mock_hive_delayed`, `.with_priority(1)`, but with a
real JSON body and a short caller-chosen delay).

**Test A — `a_stale_callback_cannot_commit_after_the_epoch_moves` (serial, browser_oauth.rs).**
Mount `mock_hive_delayed_json("POST", "/v1/oauth/web/redeem", 300, {"access_token": <jwt>,
"refresh_token": "ref"})` where `<jwt> = h.access_token_for_label("stale")` obtained BEFORE
mounting (deterministic HS256; the profile mock mounted next by `mock_hive_oauth("code-1",
"stale", "ref", owner)` matches that exact bearer). Then `start_login` in jar A; spawn the
completion GET via raw reqwest with jar A's Cookie header (clone `h.addr()` + header value
first; `use deployment::Deployment;`); await the arrival signal under 2s (claim has happened —
it precedes redeem); bump the epoch from the test
(`let mut g = h.deployment().browser_auth_epoch().lock().await; *g += 1; drop(g);`); await the
spawned response: status 400 with the generic body ("Sign-in could not be completed"), NOT the
owner-mismatch wording; `SELECT COUNT(*) FROM browser_sessions` = 0; credentials file bytes
unchanged. Mutation check: deleting the `*epoch_guard != epoch_at_claim` comparison (or
returning Ok on mismatch) must turn this test RED (200 + a session row).

**Test B — `a_credential_save_failure_mints_no_session` (serial, browser_oauth.rs).** After
`start_login`, sabotage the file backend deterministically:
`let tmp = h.credentials_path().with_extension("tmp"); let _ = std::fs::remove_file(&tmp);
std::fs::create_dir(&tmp).unwrap();` (the temp-file `open` inside `FileBackend::save` now fails
with EISDIR regardless of user). Complete the callback: status 400, generic body, NO
`vks_browser_session` Set-Cookie, `SELECT COUNT(*) FROM browser_sessions` = 0. Mutation check:
moving `create_session` before `save_credentials` must turn this RED.
## Allowed moves
[
  "Add exactly one method to remote_client.rs; change nothing else in crates/services.",
  "Create auth/login.rs and add one `pub mod login;` line.",
  "Replace the redeem/save block, remove the later detached spawn_remote_sync call, and add the Set-Cookie to the success response.",
  "Commit credentials, session and synchronous sync installation only after a matching epoch re-check under browser_auth_epoch.",
  "Append the three tests to crates/server/tests/browser_oauth.rs.",
  "Make the Remote variant's Display static (keep #[from]) and pin it with a sentinel unit test.",
  "In logout, take the share-sync handle and drop the slot guard BEFORE awaiting shutdown() (deadlock-cycle removal).",
  "Add the additive mock_hive_delayed_json helper to tests/common/mod.rs.",
  "Append tests A and B to browser_oauth.rs with the specified mutation checks."
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
  "Displaying the wrapped TokenClaimsError, token, claims or upstream body to the browser; InvalidToken has a static sanitized Display and existing generic 400 handling.",
  "Saving credentials or creating a session without re-checking epoch_at_claim under browser_auth_epoch.",
  "Holding browser_auth_epoch across Hive redemption or candidate profile I/O.",
  "Calling detached spawn_remote_sync from the successful browser-login path."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_oauth" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 011` exits 0.
2. `cargo test -p server --test browser_oauth` — 14 tests green (12 prior + tests A and B); `cargo test -p server --lib auth::login` green (sentinel Display test); `cargo test -p services` still green.
3. TS2 walk-through recorded in the ledger: name the test covering each TS2 clause (public/protected routing -> 008's suite; browser-A isolation, callback copying/replay -> 010's suite; cookie attributes, same-owner and different-owner redemption -> this suite; browser logout and explicit disconnect -> 012's suite).
4. SC5 restart clause: note that it is proven by task 015 (TS4), not here.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 011` exits 0
