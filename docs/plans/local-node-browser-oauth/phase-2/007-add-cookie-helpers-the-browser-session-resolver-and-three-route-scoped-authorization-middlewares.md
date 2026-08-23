---
id: "007"
phase: 2
title: "Add cookie helpers, the browser-session resolver and THREE route-scoped authorization middlewares"
status: ready
depends_on: ["002","005"]
parallel: false
conflicts_with: ["002","011"]
files:
  - "crates/server/src/auth/cookies.rs"
  - "crates/server/src/auth/session.rs"
  - "crates/server/src/auth/node_token.rs"
  - "crates/server/src/auth/mod.rs"
  - "crates/services/src/services/connection_token.rs"
siblings: ["crates/server/src/middleware/model_loaders.rs","crates/server/src/routes/logs.rs"]
irreversible: false
scope_test: "crates/server/src/auth/session.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
Three colocated `#[cfg(test)] mod tests`, one per new file. The pure predicate functions are what the tests drive; the thin axum wrappers are exercised end-to-end by tasks 008/013/014.

`crates/server/src/auth/cookies.rs`:
```rust
#[test]
fn session_cookie_attributes_are_exact() {
    let c = session_set_cookie("tok123");
    assert_eq!(c, "vks_browser_session=tok123; HttpOnly; SameSite=Lax; Path=/; Max-Age=157680000");
    assert!(!c.contains("Secure"), "D9: plain-HTTP LAN deployment must not set Secure");
}
#[test]
fn binding_cookie_is_lax_and_short_lived() {
    let c = binding_set_cookie("bind123");
    assert_eq!(c, "vks_browser_binding=bind123; HttpOnly; SameSite=Lax; Path=/; Max-Age=600");
    // MUST be Lax, never Strict: the hive callback is a cross-site top-level GET navigation and
    // Strict would withhold the cookie, making every login look like a wrong-browser rejection.
    assert!(c.contains("SameSite=Lax"));
}
#[test]
fn clear_cookie_expires_immediately() {
    // Byte-exact: a `; Secure` mutant (D9 violation) must fail this assertion, not pass it.
    assert_eq!(session_clear_cookie(),
        "vks_browser_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0");
    assert!(!session_clear_cookie().contains("Secure"));
}
#[test]
fn read_cookie_picks_the_named_value_from_a_multi_cookie_header() {
    let mut h = axum::http::HeaderMap::new();
    h.insert(axum::http::header::COOKIE,
        "other=1; vks_browser_session=abc; vks_browser_binding=def".parse().unwrap());
    assert_eq!(read_cookie(&h, SESSION_COOKIE), Some("abc".to_string()));
    assert_eq!(read_cookie(&h, BINDING_COOKIE), Some("def".to_string()));
    assert_eq!(read_cookie(&h, "absent"), None);
    assert_eq!(read_cookie(&axum::http::HeaderMap::new(), SESSION_COOKIE), None);
}
```

`crates/server/src/auth/session.rs` (uses `db::test_utils::create_test_pool()`):
```rust
#[tokio::test]
async fn resolver_hashes_the_presented_cookie_and_honours_revocation() {
    let (pool, _t) = db::test_utils::create_test_pool().await;
    let raw = "raw-session-token";
    let owner = uuid::Uuid::new_v4();
    db::models::browser_auth::create_session(
        &pool, uuid::Uuid::new_v4(), &crate::auth::seams::hash_token(raw), owner, 1)
        .await.unwrap();

    let mut h = axum::http::HeaderMap::new();
    h.insert(axum::http::header::COOKIE,
        format!("{}={raw}", crate::auth::cookies::SESSION_COOKIE).parse().unwrap());
    let ctx = resolve_browser_session(&pool, &h).await.expect("live session must resolve");
    assert_eq!(ctx.hive_user_id, owner);

    // Presenting the STORED HASH must NOT authorize: the server hashes what it receives.
    let mut hh = axum::http::HeaderMap::new();
    hh.insert(axum::http::header::COOKIE, format!("{}={}",
        crate::auth::cookies::SESSION_COOKIE, crate::auth::seams::hash_token(raw))
        .parse().unwrap());
    assert!(resolve_browser_session(&pool, &hh).await.is_none());

    assert!(resolve_browser_session(&pool, &axum::http::HeaderMap::new()).await.is_none());
    db::models::browser_auth::revoke_session(&pool, &crate::auth::seams::hash_token(raw), 2)
        .await.unwrap();
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
        &pool, uuid::Uuid::new_v4(), &crate::auth::seams::hash_token(raw),
        uuid::Uuid::new_v4(), 1)
        .await.unwrap();

    let mut h = axum::http::HeaderMap::new();
    h.insert(axum::http::header::COOKIE,
        format!("{}={raw}", crate::auth::cookies::SESSION_COOKIE).parse().unwrap());

    pool.close().await;
    assert!(resolve_browser_session(&pool, &h).await.is_none());
}
```

`crates/server/src/auth/node_token.rs` — the CROSS-CLASS tests are the point of this file:
```rust
#[test]
fn each_predicate_requires_its_own_audience_node_and_resource_scope() {
    let expected_node = uuid::Uuid::new_v4();
    let resource = uuid::Uuid::new_v4();
    let other = uuid::Uuid::new_v4();
    let v = ConnectionTokenValidator::new(secret());
    let conn = mint_connection_token(SECRET, expected_node, Some(resource));
    let unscoped = mint_connection_token(SECRET, expected_node, None);
    let wrong_node_conn = mint_connection_token(SECRET, other, Some(resource));
    let proxy = mint_proxy_token(SECRET, expected_node);
    let wrong_node_proxy = mint_proxy_token(SECRET, other);

    assert!(connection_token_is_valid_for_resource(
        &v, Some(&conn), expected_node, resource));
    assert!(!connection_token_is_valid_for_resource(
        &v, Some(&proxy), expected_node, resource));
    assert!(!connection_token_is_valid_for_resource(
        &v, Some(&unscoped), expected_node, resource));
    assert!(!connection_token_is_valid_for_resource(
        &v, Some(&conn), expected_node, other));
    assert!(!connection_token_is_valid_for_resource(
        &v, Some(&wrong_node_conn), expected_node, resource));

    assert!(proxy_token_is_valid_for_node(&v, Some(&proxy), expected_node));
    assert!(!proxy_token_is_valid_for_node(&v, Some(&conn), expected_node));
    assert!(!proxy_token_is_valid_for_node(&v, Some(&wrong_node_proxy), expected_node));
    assert!(!connection_token_is_valid_for_resource(&v, None, expected_node, resource));
    assert!(!proxy_token_is_valid_for_node(&v, None, expected_node));
}
```

Append service-level tests in `crates/services/src/services/connection_token.rs` proving
`validate_for_resource()` rejects `execution_process_id: None`, wrong resource and wrong node,
and `validate_proxy_for_node()` rejects a wrong target node. Mint the fixtures with `jsonwebtoken` (a crates/server dev-dependency) using the same base64 secret the validator is constructed with, matching the claim sets the validator requires: connection tokens need `sub`, `exp`, `aud="connection"`, `node_id`, `assignment_id`; proxy tokens need `sub`, `exp`, `aud="node_proxy"`, `node_id`.

The node-token test module defines its fixtures exactly from the production service tests; no undeclared `secret()`/`SECRET` placeholder remains:

```rust
use base64::{Engine as _, engine::general_purpose::STANDARD};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use secrecy::SecretString;

fn test_secret() -> SecretString {
    SecretString::from(STANDARD.encode([0x42_u8; 32]))
}
fn mint_connection_token(secret: &SecretString, node_id: Uuid, resource: Option<Uuid>) -> String {
    let now = chrono::Utc::now();
    let claims = ConnectionTokenClaims { sub: Uuid::new_v4(), node_id,
        assignment_id: Uuid::new_v4(), execution_process_id: resource,
        iat: now.timestamp(), exp: (now + chrono::Duration::minutes(15)).timestamp(),
        aud: "connection".into() };
    encode(&Header::new(Algorithm::HS256), &claims,
        &EncodingKey::from_base64_secret(secret.expose_secret()).unwrap()).unwrap()
}
fn mint_proxy_token(secret: &SecretString, target: Uuid) -> String {
    let now = chrono::Utc::now();
    let claims = ProxyTokenClaims { sub: Uuid::new_v4().to_string(),
        node_id: target.to_string(), iat: now.timestamp(),
        exp: (now + chrono::Duration::minutes(15)).timestamp(), aud: "node_proxy".into() };
    encode(&Header::new(Algorithm::HS256), &claims,
        &EncodingKey::from_base64_secret(secret.expose_secret()).unwrap()).unwrap()
}
```

Construct the validator with `ConnectionTokenValidator::new(secret.clone())`. `EncodingKey::from_secret` is forbidden here because production uses `DecodingKey::from_base64_secret`; proxy UUID claims are serialized strings by the existing `ProxyTokenClaims` type.



## Change
**File:** `crates/server/src/auth/mod.rs`
**Anchor:** the module list created by task 002.
**Before:**
```rust
pub mod seams;
```
**After:**
```rust
pub mod cookies;
pub mod node_token;
pub mod seams;
pub mod session;
```

**File:** `crates/server/src/auth/cookies.rs` — create.
```rust
/// The authorized browser-session cookie. Opaque 256-bit base64url token; only its SHA-256 hex
/// is stored server-side.
pub const SESSION_COOKIE: &str = "vks_browser_session";
/// The pre-auth handoff binding cookie. Present only between OAuth initiation and callback.
pub const BINDING_COOKIE: &str = "vks_browser_binding";

/// Five years in seconds (5 * 365 * 24 * 3600). Persistent across browser restart (SC5).
const SESSION_MAX_AGE_SECS: i64 = 157_680_000;
/// Ten minutes -- matches HANDOFF_TTL_MILLIS so a stale binding cookie cannot outlive its handoff.
const BINDING_MAX_AGE_SECS: i64 = 600;

/// Read one cookie value from a `Cookie:` header. Splits on ';', trims, matches `name=`.
pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String>;

/// `Set-Cookie` for a new authorized session.
///
/// `Secure` is deliberately ABSENT (D9): the supported deployment is plain HTTP on a trusted LAN,
/// and a Secure cookie would simply never be sent. The plaintext-session risk is documented for
/// operators in docs/configuration-customisation/browser-authorization.mdx.
pub fn session_set_cookie(token: &str) -> String;
/// `Set-Cookie` that removes the session cookie from the presenting browser (Max-Age=0).
pub fn session_clear_cookie() -> String;
/// `Set-Cookie` for the pre-auth binding secret.
///
/// `SameSite=Lax`, NEVER `Strict`: the hive OAuth callback arrives as a cross-site TOP-LEVEL GET
/// navigation. Lax sends the cookie on that navigation; Strict withholds it, and the handoff
/// claim would then fail for the RIGHTFUL browser -- indistinguishable from a wrong-browser
/// rejection.
pub fn binding_set_cookie(token: &str) -> String;
```

**File:** `crates/server/src/auth/session.rs` — create.
```rust
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
pub async fn resolve_browser_session(pool: &SqlitePool, headers: &HeaderMap)
    -> Option<BrowserSessionCtx>;

/// REJECTING layer for the protected router. 401 when no live session is presented, otherwise
/// inserts `Extension<BrowserSessionCtx>` and calls `next`.
///
/// Runs BEFORE any route-specific extractor, resource lookup or protocol upgrade, because it is
/// layered on the whole protected subtree rather than on individual handlers (D1).
pub async fn require_browser_session(State(deployment): State<DeploymentImpl>,
    request: Request, next: Next) -> Result<Response, StatusCode>;
```

**File:** `crates/server/src/auth/node_token.rs` — create. TWO route-scoped alternatives, never one
general-purpose one:
```rust
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
```

**File:** `crates/services/src/services/connection_token.rs`
**Anchor:** `validate_for_execution` and `validate_proxy_token`. Keep those existing methods for compatibility and add strict receiver-side methods:
```rust
pub fn validate_for_resource(&self, token: &str, expected_node_id: Uuid,
    expected_resource_id: Uuid) -> Result<ConnectionToken, ConnectionTokenError>;
pub fn validate_proxy_for_node(&self, token: &str, expected_node_id: Uuid)
    -> Result<ProxyToken, ConnectionTokenError>;
```
`validate_for_resource()` calls `validate()` then requires BOTH `node_id == expected_node_id` and
`execution_process_id == Some(expected_resource_id)`; `None` is not wildcard access.
`validate_proxy_for_node()` calls `validate_proxy_token()` then requires
`target_node_id == expected_node_id.to_string()`. Add explicit node/resource mismatch error
variants without placing claim values in public error text.

**File:** `crates/server/src/auth/node_token.rs` — create. TWO audience-specific predicates:
```rust
pub fn connection_token_is_valid_for_resource(validator: &ConnectionTokenValidator,
    token: Option<&str>, expected_node_id: Uuid, expected_resource_id: Uuid) -> bool;
pub fn proxy_token_is_valid_for_node(validator: &ConnectionTokenValidator,
    token: Option<&str>, expected_node_id: Uuid) -> bool;

/// Exactly the attempt-id direct diff plus raw/live direct logs. First resolve a browser session;
/// otherwise extract the nested node identity fail-closed:
/// `let runner = deployment.node_runner_context().ok_or(StatusCode::UNAUTHORIZED)?;`
/// `let expected_node_id = runner.node_id().await.ok_or(StatusCode::UNAUTHORIZED)?;`
/// then extract the route's sole UUID capture and `?token=` and call the
/// strict connection predicate. Insert BrowserSessionCtx on the browser branch. Never call next
/// for a missing/malformed/wrong-audience/wrong-node/unscoped/wrong-resource token.
pub async fn require_session_or_connection_token(State(deployment): State<DeploymentImpl>,
    Path(resource_id): Path<Uuid>, request: Request, next: Next)
    -> Result<Response, StatusCode>;

/// By-remote-id/by-task-id HTTP only, excluding diff. First resolve and insert BrowserSessionCtx;
/// otherwise obtain the current node ID and fail 401 if absent, require Authorization: Bearer,
/// and call only the strict proxy predicate. Query tokens and connection audience never pass.
pub async fn require_session_or_proxy_token(State(deployment): State<DeploymentImpl>,
    request: Request, next: Next) -> Result<Response, StatusCode>;
```
The browser branch is independent of node-runner availability. The non-browser branch fails closed
when this receiver cannot establish its own Hive node identity.

**Sibling alignment (rubric 9).** Read `crates/server/src/middleware/model_loaders.rs:272-300` before writing: it is the house `from_fn_with_state` middleware shape (State + Request + Next -> `Result<Response, StatusCode>`, `request.extensions_mut().insert(...)`, `tracing::warn!` on rejection). Match it, including logging a rejection WITHOUT logging any token or cookie value. Read `crates/services/src/services/connection_token.rs` for the two validators' exact claim requirements and audiences, and `crates/server/src/routes/logs.rs:155-190` for the existing optional-token pattern these layers supersede.

**Symbol grounding:** This task introduces `read_cookie()`, `session_set_cookie()`, `session_clear_cookie()`, `binding_set_cookie()`, `resolve_browser_session()`, `require_browser_session()`, `connection_token_is_valid_for_resource()`, `proxy_token_is_valid_for_node()`, `require_session_or_connection_token()` and `require_session_or_proxy_token()`, plus the `BrowserSessionCtx` type. It deliberately does NOT introduce any general-purpose cross-class predicate — no single "any valid node token" helper and no single combined middleware — because that is exactly the privilege widening this design rejects. `hash_token()` is defined by task 002 and only called here; `authenticate_session()` and `revoke_session()` are defined by task 005 and only called here. `validate_for_resource()` and `validate_proxy_for_node()` are introduced in the services file by this task.

**Compile-order contract for both alternative middlewares.** Resolve `BrowserSessionCtx` first; on success insert it and `return Ok(next.run(request).await)` before touching node-runner state. Only the non-browser branch evaluates:
```rust
let runner = deployment
    .node_runner_context()
    .ok_or(StatusCode::UNAUTHORIZED)?;       // outer Option<&NodeRunnerContext>
let expected_node_id = runner
    .node_id().await
    .ok_or(StatusCode::UNAUTHORIZED)?;       // inner async Option<Uuid>
```
Use this exact two-stage extraction in connection and proxy middleware. Do not chain `node_id()` directly from the deployment accessor: the accessor yields a lock containing an optional context, so the lock and both Option layers must be handled explicitly.



## Allowed moves
[
  "Create the three files and extend auth/mod.rs by exactly three `pub mod` lines.",
  "Cookie strings must be byte-exact as asserted in the tests.",
  "Keep the two strict token predicates and the two middlewares strictly separate — no shared 'any valid token' helper.",
  "Never log, format or return a raw cookie value, session token, connection token or proxy token from any of these files.",
  "Do not touch routes/*.rs in this task — wiring the layers into routers is tasks 008/013/014."
]


## STOP triggers
[
  "Collapsing the two predicates or the two middlewares into one 'any valid node token' check — that is the privilege widening this task exists to prevent: a log-stream token would open the proxy surface and vice versa.",
  "Accepting a proxy token in require_session_or_connection_token, or a connection token in require_session_or_proxy_token.",
  "Using loose `validate()` or `validate_proxy_token()` in either receiving middleware — use the strict node/resource method for that route class; the loose methods only decode audience claims.",
  "Any urge to add `Secure` to the session or binding cookie — D9 forbids it on the supported plain-HTTP LAN boundary.",
  "Any urge to set SameSite=Strict on the binding cookie — it breaks the cross-site callback navigation.",
  "Any time-based expiry check inside resolve_browser_session.",
  "Any call to Hive, the profile cache, or `deployment.get_login_status()` inside these files — authorization must be purely local.",
  "Either middleware passing a request that presented no credential at all.",
  "Adding a cookie crate or the axum-extra `cookie` feature — the parsing here is ~10 lines and a dependency change is not in this task's files: list.",
  "Using EncodingKey::from_secret for connection/proxy fixtures — the validator decodes a base64 secret; mirror test_secret() and from_base64_secret exactly.",
  "Serializing ProxyTokenClaims sub/node_id as UUID values — the existing struct requires strings.",
  "Looking up NodeRunnerContext before returning from the valid browser-session branch, or chaining node_id() directly on Option<&NodeRunnerContext>."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server auth::" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 007` exits 0.
2. `cargo test -p server auth:: && cargo test -p services connection_token` — strict audience, node and resource tests green.
3. `git grep -n 'Secure' crates/server/src/auth/cookies.rs` shows only the explanatory comment, never an emitted attribute.
4. Cross-class evidence recorded in the ledger: paste the two cross-audience assertions plus the wrong-resource assertion from `each_predicate_requires_its_own_audience_node_and_resource_scope` and the `connection_token.rs` line numbers that set each audience.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 007` exits 0
