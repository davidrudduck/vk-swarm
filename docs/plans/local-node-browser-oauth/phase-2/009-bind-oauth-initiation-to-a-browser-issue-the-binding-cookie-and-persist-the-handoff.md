---
id: "009"
phase: 2
title: "Bind OAuth initiation to a browser: issue the binding cookie and persist the handoff"
status: ready
depends_on: ["004","007","008"]
parallel: false
conflicts_with: ["008","010","011","012"]
files:
  - "crates/server/src/routes/oauth.rs"
  - "crates/server/tests/browser_oauth.rs"
siblings: ["crates/server/tests/events.rs","crates/server/tests/harness_smoke.rs","crates/server/tests/mcp_context_test.rs"]
irreversible: false
scope_test: "crates/server/tests/browser_oauth.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `crates/server/tests/browser_oauth.rs` — create.

```rust
mod common;

#[tokio::test]
#[serial_test::serial]
async fn initiation_issues_a_binding_cookie_and_persists_only_its_hash() {
    let h = common::HiveHarness::configured().await;
    let handoff_id = h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let mut jar = common::CookieJar::new();

    let res = h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider": "github", "return_to": "/"}), &mut jar).await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);

    let line = res.set_cookie.iter().find(|c| c.starts_with("vks_browser_binding="))
        .expect("no binding cookie issued");
    assert!(line.contains("HttpOnly"), "{line}");
    assert!(line.contains("SameSite=Lax"), "{line}");
    assert!(line.contains("Path=/"), "{line}");
    assert!(line.contains("Max-Age=600"), "{line}");
    assert!(!line.contains("Secure"), "D9: no Secure on the plain-HTTP LAN boundary: {line}");

    let raw = jar.get("vks_browser_binding").expect("jar did not store the cookie").to_string();
    assert!(!res.body.contains(&raw), "binding secret leaked into the response body");

    let (stored, created, expires): (String, i64, i64) = sqlx::query_as(
        "SELECT binding_hash, created_at, expires_at FROM browser_oauth_handoffs WHERE handoff_id = ?")
        .bind(handoff_id).fetch_one(h.pool()).await.unwrap();
    assert_eq!(stored, server::auth::seams::hash_token(&raw), "only the hash may be stored");
    assert_ne!(stored, raw);
    assert_eq!(expires - created, 600_000, "exactly ten minutes");

    let state: String = sqlx::query_scalar(
        "SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
        .bind(handoff_id).fetch_one(h.pool()).await.unwrap();
    assert_eq!(state, "pending");
}

#[tokio::test]
#[serial_test::serial]
async fn two_browsers_get_two_different_binding_secrets() {
    let h = common::HiveHarness::configured().await;
    h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let mut a = common::CookieJar::new();
    let mut b = common::CookieJar::fresh();
    h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut a).await;
    h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut b).await;
    assert_ne!(a.get("vks_browser_binding"), b.get("vks_browser_binding"));
}
```


## Change
**File:** `crates/server/src/routes/oauth.rs`
**Anchor:** `async fn handoff_init` (L40-67).
**Before:**
```rust
async fn handoff_init(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<HandoffInitPayload>,
) -> Result<ResponseJson<ApiResponse<HandoffInitResponseBody>>, ApiError> {
    let client = deployment.remote_client()?;

    let app_verifier = generate_secret();
    let app_challenge = hash_sha256_hex(&app_verifier);
    ...
    deployment
        .store_oauth_handoff(response.handoff_id, payload.provider, app_verifier)
        .await;

    Ok(ResponseJson(ApiResponse::success(HandoffInitResponseBody { ... })))
}
```
**After:**
```rust
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
        [(axum::http::header::SET_COOKIE, binding_set_cookie(&binding_token))],
        ResponseJson(ApiResponse::success(body)),
    )
        .into_response())
}
```
Imports to add at the top of the file: `crate::auth::cookies::binding_set_cookie`, `crate::auth::seams::{Clock, OsTokenSource, SystemClock, TokenSource, hash_token}`, `db::models::browser_auth::create_handoff`, `axum::response::IntoResponse`.

**Deliberately retained.** `LocalDeployment::store_oauth_handoff` / `take_oauth_handoff` and the in-memory `oauth_handoffs` map stay in `crates/local-deployment/src/lib.rs:743-764` but are no longer CALLED. Removing them is a public-contract change on a type this plan did not author, and this plan's only irreversible budget is task 001's migration. They are inert once task 010 stops reading them.

**File:** `crates/server/tests/browser_oauth.rs` — create, with the two tests above.

**Symbol grounding:** This task introduces no new function: it rewrites the body of the existing `handoff_init()` handler. It calls `create_handoff()` (task 004), `hash_token()` and `generate_token()` (task 002) and `binding_set_cookie()` (task 007). The pre-existing `generate_secret()` and `hash_sha256_hex()` in this file are left untouched.


## Allowed moves
[
  "Change only the body and return type of handoff_init, plus the file's import block.",
  "Create crates/server/tests/browser_oauth.rs with exactly the two tests above.",
  "Do not touch handoff_complete, logout, status, the helper fns, or the router fns in this file.",
  "Do not delete store_oauth_handoff / take_oauth_handoff from local-deployment."
]


## STOP triggers
[
  "The raw binding token appearing anywhere except the Set-Cookie header — not in the body, not in a log, not in a redirect.",
  "Storing the binding token instead of its hash.",
  "Computing expires_at at this call site instead of letting create_handoff derive it from HANDOFF_TTL_MILLIS.",
  "Using chrono::Utc::now() inline instead of the Clock seam.",
  "Any edit to crates/local-deployment/src/lib.rs — not in files:.",
  "A test needing a harness method that does not exist — STOP; harness changes belong to task 006."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_oauth" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 009` exits 0.
2. `cargo test -p server --test browser_oauth` — 2 tests green.
3. `git grep -n 'store_oauth_handoff' crates/server/` returns nothing (the call site is gone).
4. `cargo test -p server --test browser_auth_routes` still green (initiation stayed public).


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 009` exits 0
