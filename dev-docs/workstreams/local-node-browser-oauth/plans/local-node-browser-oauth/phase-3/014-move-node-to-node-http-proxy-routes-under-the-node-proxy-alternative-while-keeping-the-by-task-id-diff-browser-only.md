---
id: "014"
phase: 3
title: "Move node-to-node HTTP proxy routes under the node_proxy alternative while keeping the by-task-id diff browser-only"
status: passed
depends_on: ["013"]
parallel: false
conflicts_with: ["008","013"]
files:
  - "crates/server/src/routes/projects/mod.rs"
  - "crates/server/src/routes/task_attempts/mod.rs"
  - "crates/server/src/routes/mod.rs"
  - "crates/server/src/middleware/model_loaders.rs"
  - "crates/server/tests/proxy_auth.rs"
siblings: ["crates/server/tests/events.rs","crates/services/src/services/connection_token.rs","crates/server/tests/harness_smoke.rs","frontend/src/hooks/useDiffStream.ts","crates/server/tests/mcp_context_test.rs"]
irreversible: false
scope_test: "crates/server/tests/proxy_auth.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS3"]
---
## Failing test (write first)
File: `crates/server/tests/proxy_auth.rs` — create. Mint real Hive-signed tokens with `jsonwebtoken` and the same base64 secret placed in `VK_CONNECTION_TOKEN_SECRET`. Exercise both proxy subtrees and all three direct connection-stream routes.

```rust
// Shared harness HTTP helpers unused by a given probe would otherwise fail
// `clippy -D warnings` in this test binary (same pattern as stream_auth.rs).
#[allow(dead_code)]
mod common;

#[tokio::test]
#[serial_test::serial]
async fn proxy_http_routes_accept_browser_or_node_proxy_but_reject_missing_and_connection() {
    let _secret = with_connection_secret(SECRET);
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let proxy = mint_proxy_token(SECRET, node_id);
    let wrong_target_proxy = mint_proxy_token(SECRET, uuid::Uuid::new_v4());
    let conn = mint_connection_token(SECRET, node_id, Some(id));
    // Include the three production-prefixed wildcard/create paths. `assert_ne!(401)`
    // alone is hollow: an unregistered `/api/projects/.../files/...` falls through
    // to `api_not_found` (JSON 404), which is not 401. Pin registration by rejecting
    // the `unknown api route` body after a valid credential.
    let paths = [
        format!("/api/projects/by-remote-id/{id}/branches"),
        format!("/api/projects/by-remote-id/{id}/files/probe.txt"),
        format!("/api/task-attempts/by-task-id/{id}/branch-status"),
        format!("/api/task-attempts/by-task-id/{id}/files/probe.txt"),
        format!("/api/task-attempts/by-task-id/{id}/create"),
    ];

    for path in paths {
        assert_eq!(h.get(&path).await.status, 401,
            "{path}: missing credential must stop before lookup");
        assert_eq!(h.get_with_headers(&path,
            &[("authorization", "Bearer garbage")]).await.status, 401,
            "{path}: invalid proxy token must stop before lookup");
        assert_eq!(h.get_with_headers(&path,
            &[("authorization", &format!("Bearer {conn}"))]).await.status, 401,
            "{path}: connection audience must not open proxy HTTP");
        assert_eq!(h.get_with_headers(&path,
            &[("authorization", &format!("Bearer {wrong_target_proxy}"))]).await.status, 401,
            "{path}: wrong target node must stop before lookup");
        let proxy_ok = h.get_with_headers(&path,
            &[("authorization", &format!("Bearer {proxy}"))]).await;
        assert_ne!(proxy_ok.status, 401,
            "{path}: valid node_proxy must pass the auth boundary");
        assert!(
            !proxy_ok.body.contains("unknown api route"),
            "{path}: valid node_proxy must hit the registered /projects or /task-attempts prefix, not api_not_found"
        );

        let mut jar = h.authorized_jar().await;
        let browser_ok = h.get_with(&path, &mut jar).await;
        assert_ne!(browser_ok.status, 401,
            "{path}: browser session must bypass the inner proxy-token requirement");
        assert!(
            !browser_ok.body.contains("unknown api route"),
            "{path}: browser must hit the registered /projects or /task-attempts prefix, not api_not_found"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_tokens_fail_every_direct_log_and_direct_diff_route() {
    let _secret = with_connection_secret(SECRET);
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let proxy = mint_proxy_token(SECRET, node_id);
    for path in [format!("/api/logs/{id}/live"),
                 format!("/api/execution-processes/{id}/raw-logs/ws"),
                 format!("/api/task-attempts/{id}/diff/ws")] {
        assert_eq!(h.ws_probe(&format!("{path}?token={proxy}"), None).await.status, 401,
            "{path}: node_proxy must never open a direct stream");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn by_task_id_diff_is_browser_only_not_either_token_alternative() {
    let _secret = with_connection_secret(SECRET);
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let path = format!("/api/task-attempts/by-task-id/{id}/diff/ws");
    let conn = mint_connection_token(SECRET, node_id, Some(id));
    let proxy = mint_proxy_token(SECRET, node_id);
    assert_eq!(h.ws_probe(&path, None).await.status, 401);
    assert_eq!(h.ws_probe(&format!("{path}?token={conn}"), None).await.status, 401,
        "by-task-id diff is not the production direct connection-token URL");
    assert_eq!(h.ws_probe(&format!("{path}?token={proxy}"), None).await.status, 401,
        "proxy query token must not open a WebSocket");
    assert_eq!(h.ws_probe_with_headers(&path, None,
        &[("authorization", &format!("Bearer {proxy}"))]).await.status, 401,
        "proxy bearer token must not open a WebSocket");

    let jar = h.authorized_jar().await;
    assert_eq!(h.ws_probe(&path, Some(&jar)).await.status, 404,
        "browser passes auth; random task id is looked up only afterwards");
}

#[tokio::test]
#[serial_test::serial]
async fn disabled_validator_has_no_anonymous_or_token_fallback() {
    let h = common::HiveHarness::configured().await;
    let path = format!("/api/projects/by-remote-id/{}/branches", uuid::Uuid::new_v4());
    assert_eq!(h.get(&path).await.status, 401);
    assert_eq!(h.get_with_headers(&path,
        &[("authorization", &format!("Bearer {}", mint_proxy_token(SECRET, uuid::Uuid::new_v4())))])
        .await.status, 401);
}
```
Use task 006's real `ws_probe_with_headers()` for the bearer-token attempt; do not simulate a WebSocket with an ordinary GET. `with_connection_secret`, `mint_connection_token` and `mint_proxy_token` remain local RAII/fixture helpers — the three `configured_with_node_auth` tests MUST hold `let _secret = with_connection_secret(SECRET)` so Drop clears `VK_CONNECTION_TOKEN_SECRET` before `disabled_validator_has_no_anonymous_or_token_fallback` (which uses `configured()` and must not inherit a leftover secret). The first test's browser assertion is load-bearing: it fails unless the existing model loaders recognize the `BrowserSessionCtx` inserted by the outer alternative middleware. The `"unknown api route"` body pin is load-bearing: `api_not_found` is a JSON 404, so `assert_ne!(401)` and `assert_registered()` (SPA-html oracle) both stay green when files/create lose their `/projects` or `/task-attempts` parent nest.


## Change
**File:** `crates/server/src/routes/projects/mod.rs`
**Anchor:** `projects_router` at L87-141. Move the complete `/by-remote-id/{remote_project_id}` router and wildcard files router out of `router()` into `node_to_node_router(deployment)`, verbatim with `load_project_by_remote_id_middleware`. Leave local `/{id}` project routes in `router()`.
```rust
/// Node-to-node project HTTP. Browser session OR node_proxy, never connection.
pub fn node_to_node_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Existing by_remote_id_router and by_remote_id_files_router, moved without handler changes.
    // MUST keep the old parent nest so files stay at /projects/by-remote-id/{id}/files/{*}.
    // Merging the files router at the API root drops the /projects prefix and 404s
    // crates/server/src/routes/projects/handlers/files.rs:112.
    Router::new().nest(
        "/projects",
        Router::new()
            .nest("/by-remote-id/{remote_project_id}", by_remote_id_router)
            .merge(by_remote_id_files_router),
    )
}
```

**File:** `crates/server/src/routes/task_attempts/mod.rs`
**Anchor 1:** `by_task_id_router` at L146-180. Remove `.route("/diff/ws", get(stream_task_attempt_diff_ws))` from this proxy-shaped router. Re-register that one route, with the SAME `load_task_attempt_by_task_id_middleware`, in a small router returned from ordinary `router()` so it remains behind `require_browser_session` only.

**Anchor 2:** `task_attempts_router` at L205-211. Move the remaining by-task-id router, wildcard files router and create router into:
```rust
/// Node-to-node task-attempt HTTP only. Deliberately excludes every WebSocket.
pub fn node_to_node_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Existing by_task_id_router minus /diff/ws, plus files/create routers, moved verbatim.
    // MUST keep the old parent nest so files/create stay under /task-attempts/by-task-id/{id}/...
    // Merging those routers at the API root drops the prefix and 404s
    // worktree.rs:182 and core.rs:390.
    Router::new().nest(
        "/task-attempts",
        Router::new()
            .nest("/by-task-id/{task_id}", by_task_id_router)
            .merge(by_task_id_files_router)
            .merge(by_task_id_create_router),
    )
}
```
The production direct diff `/task-attempts/{id}/diff/ws` was already moved to `connection_stream_routes` by task 013. The unrelated `/by-task-id/{task_id}/diff/ws` has no production direct caller and belongs to neither token group.

**File:** `crates/server/src/middleware/model_loaders.rs`
**Anchors:** `load_project_by_remote_id_middleware` L272-312, `load_task_attempt_by_task_id_impl` L754-792, and `load_task_by_task_id_impl` L866-900.

Each loader currently gates its proxy check with `if validator.is_enabled()`. That creates two defects after the outer OR middleware is added: an anonymous request falls through when the validator is disabled, and a valid browser session is rejected for lacking a bearer token when it is enabled. Replace each guard with the same explicit branch:
```rust
    // The outer route-class middleware has already authenticated one of two alternatives.
    // Browser sessions carry BrowserSessionCtx and need no proxy token. Non-browser requests
    // must still be revalidated here so the existing ProxyRequestContext/binding behavior stays.
    if request.extensions().get::<BrowserSessionCtx>().is_none() {
        let token = extract_bearer_token(request.headers()).ok_or(StatusCode::UNAUTHORIZED)?;
        let expected_node_id = deployment.node_runner_context()
            .ok_or(StatusCode::UNAUTHORIZED)?
            .node_id().await.ok_or(StatusCode::UNAUTHORIZED)?;
        // Keep the existing match arms and ProxyRequestContext insertion verbatim, but call:
        let proxy = deployment.connection_token_validator()
            .validate_proxy_for_node(token, expected_node_id)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
    }
```
Import `crate::auth::session::BrowserSessionCtx`. Do NOT remove the proxy-token decode, source/target logging, `ProxyRequestContext` insertion or any database lookup. The outer middleware enforces class before this loader; this loader retains proxy binding before its lookup. With a browser context it skips only redundant proxy authentication, then performs the same resource lookup.

**File:** `crates/server/src/routes/mod.rs`
**Anchor:** `connection_stream_routes` from task 013. Add a separate group:
```rust
    let node_to_node_routes = Router::new()
        .merge(projects::node_to_node_router(&deployment))
        .merge(task_attempts::node_to_node_router(&deployment))
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::auth::node_token::require_session_or_proxy_token,
        ));

    let base_routes = public_routes
        .merge(protected_routes)
        .merge(connection_stream_routes)
        .merge(node_to_node_routes)
        .fallback(api_not_found)
        .with_state(deployment);
```
Never merge this with connection_stream_routes and never place a WebSocket in node_to_node_routes.

**Route classification proof.** `frontend/src/hooks/useDiffStream.ts` constructs the attempt-id direct route, not the by-task-id route. Server proxy clients under `task_attempts/handlers/` construct by-task-id HTTP paths for files, branch operations, follow-up, review and PR work, but no diff WebSocket. Therefore node_proxy remains HTTP-only and the by-task-id diff is browser-only.

**Sibling alignment (rubric 9).** Read all three named loader bodies before editing and list every retained validation, context insertion and lookup-order choice in the ledger. Read `services/src/services/connection_token.rs:155-220` for the node_proxy audience. Any divergence requires ledger justification.

**Symbol grounding:** This task introduces `node_to_node_router()` in `routes/projects/mod.rs` and `routes/task_attempts/mod.rs`, plus the `node_to_node_routes` group. It calls `require_session_or_proxy_token()`, defined by task 007, and never calls `require_session_or_connection_token()`. `BrowserSessionCtx` is defined by task 007 and only read here. Test-local `mint_connection_token()` and `mint_proxy_token()` create node/resource-scoped opposite-audience fixtures. `ws_probe_with_headers()` is defined by task 006 and supplies the valid bearer-bearing WebSocket handshake.

Preserve task 013's direct-route loaders and task 014's browser-versus-node_proxy loader bypass exactly: strict target-node validation remains on non-browser proxy requests and neither JWT audience crosses route classes.



## Allowed moves
[
  "Move by-remote-id and by-task-id HTTP registrations into node_to_node_router functions with their existing loaders; exclude the by-task-id diff WebSocket.",
  "Keep /by-task-id/{task_id}/diff/ws in the ordinary browser-session-only router and keep the attempt-id direct diff in task 013's connection group.",
  "Change only the authentication guards in the three proxy loaders so BrowserSessionCtx skips bearer revalidation while non-browser requests still validate node_proxy against this receiver's current node ID before lookup.",
  "Add node_to_node_routes as a layer separate from connection_stream_routes and create proxy_auth.rs with both proxy subtrees and both cross-class directions covered.",
  "Do not change proxy handler behavior, resource lookup behavior, or ProxyRequestContext contents; the only claim-validation change is requiring the existing target node claim to equal this receiver.",
  "Nest each node_to_node_router under the same /projects or /task-attempts parent the old router() used, so wildcard files and create keep their production prefixes.",
  "Hold with_connection_secret in every configured_with_node_auth test and pin files/create registration with the unknown-api-route body check."
]


## STOP triggers
[
  "Putting any WebSocket in node_to_node_routes or accepting node_proxy on either diff URL.",
  "Putting `/task-attempts/by-task-id/{task_id}/diff/ws` in connection_stream_routes — only the attempt-id URL receives the production connection token.",
  "Merging node_to_node_routes with connection_stream_routes or introducing any predicate that accepts both audiences.",
  "Removing proxy-token revalidation/ProxyRequestContext insertion from model_loaders instead of bypassing it only for an authenticated BrowserSessionCtx.",
  "A missing, malformed or connection-audience credential reaching a proxy resource lookup and returning 404 instead of 401.",
  "A valid browser session returning 401 from a proxy route when VK_CONNECTION_TOKEN_SECRET is set — the inner loader guard is still wrong.",
  "Editing crates/server/tests/common/mod.rs; token fixtures remain local to proxy_auth.rs.",
  "Calling loose validate_proxy_token in a receiving loader — use validate_proxy_for_node so a token minted for another node returns 401.",
  "Mounting by-remote-id files or by-task-id files/create at the API root (missing the /projects or /task-attempts parent nest). Production callers construct /api/projects/by-remote-id/{id}/files/{*} and /api/task-attempts/by-task-id/{id}/files|{create}."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test stream_auth && cargo test -p server --test proxy_auth" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 014` exits 0.
2. `cargo test -p server` is green.
3. Record a route census: every by-remote-id/by-task-id HTTP registration is in node_to_node_routes; `/by-task-id/{task_id}/diff/ws` is in protected_routes; `/task-attempts/{id}/diff/ws` is in connection_stream_routes.
4. Paste evidence that connection tokens return 401 on representative project and task proxy routes, and proxy tokens return 401 on all three direct log/diff routes.
5. With VK_CONNECTION_TOKEN_SECRET set, show both a browser session and node_proxy token pass each representative proxy auth boundary; missing/garbage credentials return 401 before random IDs can return 404.
6. Confirm router construction succeeds and no route is registered in two groups.
7. TS3 ownership closes here: rerun both stream_auth and proxy_auth; task 013 alone intentionally claims only SC2 because proxy HTTP is not wired until this task.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 014` exits 0
