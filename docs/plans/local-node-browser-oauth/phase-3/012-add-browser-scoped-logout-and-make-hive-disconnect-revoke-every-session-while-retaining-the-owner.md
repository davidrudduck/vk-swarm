---
id: "012"
phase: 3
title: "Add browser-scoped logout and make Hive disconnect revoke every session while retaining the owner"
status: ready
depends_on: ["011"]
parallel: false
conflicts_with: ["008","009","010","011","022"]
files:
  - "crates/server/src/routes/oauth.rs"
  - "crates/server/tests/browser_auth_routes.rs"
irreversible: false
scope_test: "crates/server/tests/browser_auth_routes.rs"
allowed_change: edit
covers_criteria: ["SC7","SC8"]
covers_tests: []
---
## Failing test (write first)
Append four serial tests to `crates/server/tests/browser_auth_routes.rs`; use the existing local `login` helper and task 006's existing `deployment()` accessor.

```rust
#[tokio::test]
#[serial_test::serial]
async fn browser_logout_revokes_the_presented_raw_token_only_and_keeps_real_sync() {
    let h = common::HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let mut a = login(&h, owner, "code-a").await;
    let mut b = login(&h, owner, "code-b").await;
    let raw_a = a.get("vks_browser_session").expect("session cookie").to_owned();
    let hash_a = server::auth::seams::hash_token(&raw_a);
    assert_eq!(h.get_with("/api/info", &mut a).await.status, 200);

    let share = services::services::share::config::ShareConfig::from_env().expect("harness share config");
    h.deployment().spawn_remote_sync(share);
    wait_until_sync_slot_is_some(h.deployment()).await;

    let out = h.post_with("/api/auth/browser/logout", json!({}), &mut a).await;
    assert!(matches!(out.status, 200 | 204));
    assert!(out.set_cookie.iter().any(|v| v.starts_with("vks_browser_session=") && v.contains("Max-Age=0")));

    let mut replay = common::CookieJar::fresh();
    replay.insert("vks_browser_session", &raw_a);
    assert_eq!(h.get_with("/api/info", &mut replay).await.status, 401,
        "replay the captured raw token, not the now-empty presenting jar");
    let revoked_at: Option<i64> = sqlx::query_scalar(
        "SELECT revoked_at FROM browser_sessions WHERE token_hash = ?")
        .bind(hash_a).fetch_one(h.pool()).await.unwrap();
    assert!(revoked_at.is_some());
    assert_eq!(h.get_with("/api/info", &mut b).await.status, 200);
    assert!(h.deployment().share_sync_handle().lock().await.is_some(),
        "browser logout must leave the real sync handle running");
}

#[tokio::test]
#[serial_test::serial]
async fn hive_disconnect_revokes_all_sessions_stops_real_sync_and_keeps_owner() {
    let h = common::HiveHarness::configured().await;
    let owner = Uuid::new_v4();
    let mut a = login(&h, owner, "code-a").await;
    let mut b = login(&h, owner, "code-b").await;
    let share = ShareConfig::from_env().expect("harness share config");
    h.deployment().spawn_remote_sync(share);
    wait_until_sync_slot_is_some(h.deployment()).await;

    let res = h.post_with("/api/auth/logout", json!({}), &mut a).await;
    assert!(matches!(res.status, 200 | 204));
    assert!(h.deployment().share_sync_handle().lock().await.is_none());
    assert_eq!(h.get_with("/api/info", &mut a).await.status, 401);
    assert_eq!(h.get_with("/api/info", &mut b).await.status, 401);
    assert!(!h.credentials_path().exists());
    assert_eq!(stored_owner_uuid(h.pool()).await, owner);
    assert_eq!(live_session_count(h.pool()).await, 0);
}
```

The other two tests retain the different-subject-after-disconnect rejection and anonymous browser-logout 401. `wait_until_sync_slot_is_some` is a short bounded yield/timeout around `share_sync_handle().lock().await.is_some()` because `spawn_remote_sync` installs the real handle asynchronously. It MUST call public `Deployment::spawn_remote_sync(ShareConfig)`; no private `RemoteSync::spawn`, fake handle, or direct slot mutation.

Append four integrated race tests, each wrapped in `tokio::time::timeout(Duration::from_secs(30),
async { ... })` so a lock regression fails instead of hanging CI:

1. `disconnect_during_an_in_flight_callback_leaves_no_session_credentials_or_sync`: authorize
   browser B, initiate browser A with label `delayed-a`, call `delay_hive_profile` for that label,
   spawn A's callback, and await the arrival signal under a 2-second diagnostic watchdog. While
   the valid profile response is delayed, B calls `/api/auth/logout`. Await both operations. A's
   callback must return the generic 400; no session row for A exists, all previously live sessions
   are revoked, credentials are absent, sync slot is `None`, and the pinned owner remains. Mutation
   proof: removing the epoch re-check makes A return 200 and leaves credentials/session/sync live.
2. `a_pending_callback_from_before_disconnect_is_durably_invalidated`: initiate A but do not call
   its callback; disconnect through B; then call A's callback and assert 400, zero live sessions,
   no credentials, and persisted handoff state terminal. Mutation proof: remove
   `invalidate_pending_handoffs` and the callback succeeds.
3. `a_fresh_login_after_disconnect_still_succeeds`: disconnect B, perform a wholly new init and
   callback for the same pinned owner, and assert 200, one live session, credentials present and
   sync installed. This prevents the epoch/invalidation fix from permanently locking the node.
4. `disconnect_is_not_undone_by_an_in_flight_token_refresh`: arrange near-expiry credentials,
   install a delayed priority-1 refresh response and await its arrival, then call disconnect. The
   request must finish after the refresh guard becomes available and the final credentials path
   must be absent. Never take `refresh_guard` before `client.logout`.

The first test is the incident-symptom assertion for the integrated finding: after disconnect has
returned, an earlier OAuth callback cannot recreate any browser authorization or daemon state.


## Change
**File:** `crates/server/src/routes/oauth.rs`

**Anchor 1 — `pub fn protected_router()` (added by task 008).**
**Before:**
```rust
pub fn protected_router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/auth/logout", post(logout))
        .route("/auth/status", get(status))
}
```
**After:**
```rust
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
```

**Anchor 2 — new handler, placed immediately before `async fn logout`.**
```rust
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
        revoke_session(&deployment.db().pool, &hash_token(&raw), SystemClock.now_millis())
            .await
            .map_err(ApiError::Database)?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(axum::http::header::SET_COOKIE, session_clear_cookie())],
    )
        .into_response())
}
```

**Anchor 3 — `async fn logout` (`oauth.rs:168-189`), the ORDER of its steps.**
**Before:**
```rust
async fn logout(State(deployment): State<DeploymentImpl>) -> Result<StatusCode, ApiError> {
    // Stop remote sync if running
    if let Some(handle) = deployment.share_sync_handle().lock().await.take() {
        tracing::info!("Stopping remote sync due to logout");
        handle.shutdown().await;
    }
    ...
```
**After:** serialize the complete explicit-disconnect operation with browser-login commit. Increment
the epoch, durably invalidate pending handoffs, and revoke every session before any sync or
credential side effect:
```rust
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
    tracing::info!(invalidated, revoked,
        "invalidated pending logins and revoked all browser sessions for hive disconnect");

    // Stop remote sync if running
    ... (existing handle take/shutdown and client.logout remain) ...

    // Serialize only credential clearing against token refresh. Do NOT take this guard before
    // client.logout(): that call may itself refresh and tokio Mutex is not re-entrant.
    let refresh_guard = deployment.auth_context().refresh_guard().await;
    deployment.auth_context().clear_credentials().await ...;
    drop(refresh_guard);
    ... (clear profile; return while epoch_guard remains held) ...
```
Imports to add: `crate::auth::cookies::{SESSION_COOKIE, session_clear_cookie}`, `db::models::browser_auth::{invalidate_pending_handoffs, revoke_all_sessions, revoke_session}`.

**File:** `crates/server/tests/browser_auth_routes.rs` — append the four tests plus the local `login` helper.

**Symbol grounding:** This task introduces the `browser_logout()` handler and adds the `/auth/browser/logout` route to `protected_router()`. `read_cookie()` and `session_clear_cookie()` are defined by task 007, `hash_token()` by task 002, `revoke_session()` / `revoke_all_sessions()` by task 005, and function `invalidate_pending_handoffs()` by corrective task 022; this task only calls them.

**Sync test precondition.** Task 006 already exposes `HiveHarness::deployment()`. Tests construct `ShareConfig::from_env()` from the configured Wiremock base and call the public `deployment.spawn_remote_sync(config)`, then wait boundedly until `share_sync_handle().lock().await` is `Some`. Browser logout must leave that real handle `Some`; Hive disconnect must take/shutdown it and leave `None`. Do not call the private `RemoteSync` constructor and do not install a fake handle.



## Allowed moves
[
  "Add one route line and one new handler to routes/oauth.rs; serialize logout with browser_auth_epoch, bump it, invalidate pending handoffs, revoke sessions, stop sync, and clear credentials under refresh_guard.",
  "Append the four tests and the login helper to crates/server/tests/browser_auth_routes.rs.",
  "Do not rename /auth/logout, do not change its existing sync-stop or credential-clear steps, and do not touch handoff_init / handoff_complete / status."
]


## STOP triggers
[
  "browser_logout touching sync, credentials, node_owner, or any session other than the presenting one.",
  "logout deleting or clearing node_owner — the owner must survive disconnect (D4/SC8).",
  "Revoking sessions AFTER credential deletion, or making the revoke conditional on the credential delete succeeding.",
  "Renaming /api/auth/logout in this task — the backward-compatibility choice is deliberate and its caller (frontend/src/lib/api/oauth.ts) is repointed off it by task 017; a rename here would change behaviour for an unmigrated caller in the same commit.",
  "browser_logout returning 200 for a request with no session — the route is protected and must 401 first.",
  "Using the emptied presenting CookieJar after logout as the sole revocation proof — capture the raw token before logout, prove 200, replay it from a fresh jar for 401, and query revoked_at by its hash.",
  "Asserting sync scope without first installing a real handle through public spawn_remote_sync(ShareConfig). Browser logout keeps Some; disconnect reaches None.",
  "Releasing browser_auth_epoch before session revocation, sync shutdown and credential clearing complete.",
  "Taking refresh_guard before client.logout — it can re-enter refresh_guard and deadlock.",
  "Disconnect leaving a pre-existing pending handoff claimable."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_auth_routes" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 012` exits 0.
2. `cargo test -p server --test browser_auth_routes` — 12 tests green, including raw-token replay, real sync-handle scope and the four barrier-controlled disconnect races.
3. SC7/SC8 walk-through in the ledger: for each clause (scope of revocation, credentials untouched vs removed, sync stopped, owner retained) name the asserting line.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 012` exits 0
