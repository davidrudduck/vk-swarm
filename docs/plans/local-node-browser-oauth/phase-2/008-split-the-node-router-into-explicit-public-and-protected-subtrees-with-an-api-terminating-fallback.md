---
id: "008"
phase: 2
title: "Split the node router into explicit public and protected subtrees with an API-terminating fallback"
status: ready
depends_on: ["006","007"]
parallel: false
conflicts_with: ["006","009","010","011","012","013","014"]
files:
  - "crates/server/src/routes/mod.rs"
  - "crates/server/src/routes/oauth.rs"
  - "crates/server/src/routes/browser_auth.rs"
  - "crates/server/src/bin/generate_types.rs"
  - "crates/server/tests/harness_smoke.rs"
  - "shared/types.ts"
  - "crates/server/tests/browser_auth_routes.rs"
siblings: ["crates/server/src/routes/health.rs","crates/server/src/routes/config.rs","crates/server/src/routes/all_tasks.rs","crates/server/tests/events.rs","crates/server/src/routes/approvals.rs","crates/server/tests/mcp_context_test.rs","crates/server/src/routes/backups.rs","crates/server/tests/nodes_routes.rs"]
irreversible: false
scope_test: "crates/server/tests/browser_auth_routes.rs"
allowed_change: mixed
covers_criteria: ["SC1"]
covers_tests: []
---
## Failing test (write first)
File: `crates/server/tests/browser_auth_routes.rs` — create. Every test `#[serial_test::serial]`.

```rust
mod common;

#[tokio::test]
#[serial_test::serial]
async fn public_surface_is_reachable_without_a_session() {
    let h = common::HiveHarness::configured().await;
    let mut jar = common::CookieJar::new();

    let health = h.get_with("/api/health", &mut jar).await;
    health.assert_registered();
    assert_eq!(health.status, 200);

    let state = h.get_with("/api/auth/state", &mut jar).await;
    state.assert_registered();
    assert_eq!(state.status, 200, "a clean browser MUST get 200, not 401: {}", state.body);
    assert!(state.body.contains("\"authorized\":false"), "body: {}", state.body);
    assert!(state.body.contains("oauth_available"), "body: {}", state.body);
    // Minimal means minimal: no config, no environment, no profile.
    for leak in ["executor", "profile", "git_repo_path", "os_type", "user_id"] {
        assert!(!state.body.contains(leak), "auth state leaked {leak}: {}", state.body);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn protected_api_is_denied_by_default() {
    let h = common::HiveHarness::configured().await;
    for path in ["/api/info", "/api/projects", "/api/tasks/all", "/api/auth/status",
                 "/api/diagnostics", "/api/config", "/api/organizations"] {
        let res = h.get(path).await;
        res.assert_registered();
        assert_eq!(res.status, 401, "{path} must be 401 without a session; body: {}", res.body);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn unknown_api_paths_terminate_inside_the_api_boundary() {
    let h = common::HiveHarness::configured().await;
    let res = h.get("/api/definitely-not-a-route").await;
    assert!(!res.is_spa_fallback(),
        "unknown /api/* fell through to SPA HTML (status {}, ct {:?})", res.status, res.content_type);
    assert_eq!(res.status, 404);
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_initiation_and_callback_stay_public() {
    let h = common::HiveHarness::configured().await;
    h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let init = h.post("/api/auth/handoff/init",
        serde_json::json!({"provider": "github", "return_to": "/"})).await;
    init.assert_registered();
    // Handler-specific pinning: the public JSON-404 catch-all answers `{"success":false,
    // "message":"unknown api route"}` with 404 when a route is dropped, so `!= 401` alone
    // cannot prove registration (STOP trigger: status-code-alone proves routing).
    assert_eq!(init.status, 200, "body: {}", init.body);
    assert!(init.body.contains("handoff_id"), "body: {}", init.body);
    // No app_code: the registered handler answers 400 with its own message; a dropped route
    // would answer 404 JSON. No `assert_registered` here: this handler answers with HTML,
    // which `is_spa_fallback()` would misread.
    let cb = h.get("/api/auth/handoff/complete?handoff_id="
        .to_string() + &uuid::Uuid::new_v4().to_string()).await;
    assert_eq!(cb.status, 400, "body: {}", cb.body);
    assert!(cb.body.contains("Missing app_code"), "body: {}", cb.body);
}
```


## Change
**File:** `crates/server/src/routes/browser_auth.rs` — create. The minimal PUBLIC auth-state route:
```rust
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

async fn auth_state(State(deployment): State<DeploymentImpl>, headers: HeaderMap)
    -> ResponseJson<ApiResponse<BrowserAuthState>> {
    let authorized = crate::auth::session::resolve_browser_session(
        &deployment.db().pool, &headers).await.is_some();
    // `remote_client()` is Err only when the node has no hive configured; a hive OUTAGE does not
    // change this flag, and neither flag depends on hive reachability (SC9).
    let oauth_available = deployment.remote_client().is_ok();
    ResponseJson(ApiResponse::success(BrowserAuthState { authorized, oauth_available }))
}
```

**File:** `crates/server/src/routes/oauth.rs`
**Anchor:** `pub fn router()` at L22-28.
**Before:**
```rust
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/auth/handoff/init", post(handoff_init))
        .route("/auth/handoff/complete", get(handoff_complete))
        .route("/auth/logout", post(logout))
        .route("/auth/status", get(status))
}
```
**After:**
```rust
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
        .route("/auth/logout", post(logout))
        .route("/auth/status", get(status))
}
```
Handler bodies are NOT touched in this task.

**File:** `crates/server/src/routes/mod.rs`
**Anchor 1:** the `pub mod` list — add `pub mod browser_auth;` between `pub mod breakdown;` and `pub mod config;` (alphabetical).
**Anchor 2:** `pub async fn router(...)`, the `let base_routes = Router::new()...` block (L47-80).
**Before:**
```rust
    let base_routes = Router::new()
        .route("/health", get(health::health_check))
        .merge(config::router())
        ... (the full merge chain) ...
        .merge(terminal_router)
        .nest("/images", images::routes())
        .with_state(deployment);
```
**After:**
```rust
    // Deny-by-default (D1): every route lives in exactly one of these two subtrees, and anything
    // added to `protected_routes` in future inherits authorization without opting in.
    let public_routes = Router::new()
        .route("/health", get(health::health_check))
        .merge(oauth::public_router())
        .merge(browser_auth::public_router());

    let protected_routes = Router::new()
        .merge(config::router())
        ... (every OTHER entry from the original chain, unchanged and in the same order,
             with `.merge(oauth::router())` replaced by `.merge(oauth::protected_router())`) ...
        .merge(terminal_router)
        .nest("/images", images::routes())
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::auth::session::require_browser_session,
        ));

    let base_routes = public_routes
        .merge(protected_routes)
        // An unknown `/api/*` request must terminate INSIDE the API boundary. Without this
        // fallback the nest misses and the outer `/{*path}` catch-all serves SPA index.html with
        // 200 OK -- see Resp::is_spa_fallback in crates/server/tests/common/mod.rs.
        .fallback(api_not_found)
        .with_state(deployment);
```

**Axum reality (correction, verified empirically on axum 0.8.8):** a nested custom `fallback` is
filed under the PARENT router's fallback router, and the outer `/{*path}` SPA catch-all is a REAL
route — real routes win over fallbacks, so `.fallback(api_not_found)` alone is SHADOWED and the
test stays RED (observed). The JSON 404 must ALSO be registered as a catch-all route inside the
nest, immediately after the two subtrees are merged and BEFORE `.with_state`:

```rust
    let base_routes = public_routes
        .merge(protected_routes)
        .route("/{*path}", any(api_not_found))
        .fallback(api_not_found)
        .with_state(deployment);
```

(`any` = `axum::routing::any`. The catch-all sits outside `protected_routes`, so it is public JSON
404 — it must not leak an auth distinction. Mutation check: removing only the `.route(...)`
line makes `unknown_api_paths_terminate_inside_the_api_boundary` fail with SPA HTML.)
**Anchor 3:** add, after `pub async fn router`:
```rust
/// 404 for any unmatched path under `/api`, as JSON, so the SPA catch-all can never answer for
/// an API call.
async fn api_not_found() -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::NOT_FOUND,
     axum::Json(serde_json::json!({"success": false, "message": "unknown api route"})))
}
```

**File:** `crates/server/src/bin/generate_types.rs`
**Anchor:** the explicit `let decls: Vec<String> = vec![ ... ]` list inside `generate_types_content()` (starts L13). `#[derive(TS)]` alone is NOT enough in this repo — a type absent from this list simply never reaches `shared/types.ts`.
**Before:** the existing entry `server::routes::projects::UnifiedProject::decl(),` (or any neighbouring line — placement within the vec is free).
**After:** add one line, grouped with the other `server::routes::` entries:
```rust
        server::routes::browser_auth::BrowserAuthState::decl(),
```

**File:** `shared/types.ts`
**Anchor:** generated file — do not hand-edit.
**Change:** run `npm run generate-types` after adding `#[derive(TS)]` to `BrowserAuthState` AND registering it in the decl list above, then commit the resulting `BrowserAuthState` addition (CLAUDE.md §7: Rust types are the source of truth). Note that `generate_types` WIPES and rewrites the whole `shared/` directory including `shared/schemas/`; if anything other than the new type changes, that is pre-existing drift — stop and report it rather than committing it.

**Sibling alignment (rubric 9).** `browser_auth.rs` is a NEW simple route module: match the shape of `crates/server/src/routes/health.rs` (single handler, `State<DeploymentImpl>`, no ts-rs types) and `crates/server/src/routes/config.rs:38-55` (`pub fn router() -> Router<DeploymentImpl>` plus `#[derive(Serialize, Deserialize, TS)]` response structs). Return `ApiResponse::success(...)` like every other route (CLAUDE.md §7).

**File:** `crates/server/tests/harness_smoke.rs`
**Anchor:** the second half of `harness_detects_an_unregistered_route` (L51-59) — this assertion INVERTS, because terminating unknown `/api/*` inside the API boundary is the point of SC1.
**Before:**
```rust
    // A path that is NOT registered. It returns 200 + SPA HTML, NOT 404 — which is
    // exactly why assert_ne!(404) cannot prove registration in this codebase.
    let missing = h.get("/api/definitely-not-a-route").await;
    assert!(
        missing.is_spa_fallback(),
        "expected the SPA fallback for an unregistered route, got status {} body {:.80}",
        missing.status,
        missing.body
    );
```
**After:**
```rust
    // A path that is NOT registered. Since the public/protected split landed, an unmatched
    // `/api/*` path terminates on the API router's own fallback and NEVER reaches the SPA
    // catch-all. `is_spa_fallback()` remains the registration oracle for NON-api paths.
    let missing = h.get("/api/definitely-not-a-route").await;
    assert!(
        !missing.is_spa_fallback(),
        "unknown /api path fell through to SPA HTML, status {} body {:.80}",
        missing.status,
        missing.body
    );
    assert_eq!(missing.status, 404);
```
Nothing else in `harness_smoke.rs` changes: task 006 already repointed its protected-route calls through `authorized_jar()`.

**Symbol grounding:** This task introduces `public_router()` and `protected_router()` in `routes/oauth.rs`, and `public_router()`, the `auth_state()` handler and the `BrowserAuthState` type in the new `routes/browser_auth.rs`, plus `api_not_found()` in `routes/mod.rs`. It calls `resolve_browser_session()` and `require_browser_session()`, both defined by task 007.


## Allowed moves
[
  "Split routes/oauth.rs's router() into public_router() and protected_router(); move NO handler code.",
  "Create routes/browser_auth.rs with exactly the state struct, public_router() and the auth_state handler.",
  "Regroup the existing merge chain in routes/mod.rs into public_routes/protected_routes, add the layer, the fallback and api_not_found. Every merge entry must survive, in the same relative order.",
  "Register the JSON 404 both as a catch-all route inside the nest (`.route(\"/{*path}\", any(api_not_found))`) and as the nested fallback — the outer `/{*path}` SPA route shadows fallback-only registration (axum 0.8.8).",
  "Regenerate shared/types.ts via `npm run generate-types`.",
  "Append the four tests to the new crates/server/tests/browser_auth_routes.rs."
]


## STOP triggers
[
  "`/api/auth/state` ends up behind require_browser_session — it must answer 200 with authorized:false for a clean browser; that is SC1's other half.",
  "Any route from the original merge chain is dropped or silently reordered relative to its neighbours.",
  "`npm run generate-types` producing changes under shared/schemas/ or to any type other than BrowserAuthState — STOP and report; the generator wipes and rewrites all of shared/, so an unrelated diff means pre-existing drift, not your change.",
  "`npm run generate-types` rewrites types other than BrowserAuthState — STOP: that is pre-existing type drift and must be reported, not swept into this commit.",
  "A test asserts a status code alone to prove routing — use Resp::assert_registered()/is_spa_fallback(), because the SPA catch-all answers 200 for any NON-api path.",
  "Cross-node features break here and STAY broken: the by-remote-id / by-task-id proxy subtrees and the Hive direct-log streams and attempt-id direct-diff stream now sit behind the browser-session layer. That is INTENDED and is undone by tasks 013/014. Do not weaken the layer to keep them working, and do not skip 013/014.",
  "Any handler body inside routes/oauth.rs changing in this task.",
  "`cargo test -p server` red for any reason other than the harness_smoke inversion handled above — STOP; task 006 was supposed to have repointed every consumer."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_auth_routes" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 008` exits 0.
2. `cargo test -p server --test browser_auth_routes` — 4 tests green.
3. `cargo test -p server` — whole suite green (006's repoint plus this task's harness_smoke inversion).
4. `npm run generate-types:check` exits 0.
5. Record in the ledger the explicit list of routes now in `public_routes`, and confirm nothing else is in it.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 008` exits 0
