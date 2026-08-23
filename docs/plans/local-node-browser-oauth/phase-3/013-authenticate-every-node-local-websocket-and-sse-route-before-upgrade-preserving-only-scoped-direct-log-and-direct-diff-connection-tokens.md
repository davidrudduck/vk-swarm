---
id: "013"
phase: 3
title: "Authenticate every node-local WebSocket and SSE route before upgrade, preserving only scoped direct-log and direct-diff connection tokens"
status: ready
depends_on: ["008","011"]
parallel: false
conflicts_with: ["008","014"]
files:
  - "crates/server/src/routes/mod.rs"
  - "crates/server/src/routes/logs.rs"
  - "crates/server/src/routes/execution_processes.rs"
  - "crates/server/src/routes/task_attempts/mod.rs"
  - "crates/server/src/routes/task_attempts/types.rs"
  - "crates/server/src/routes/task_attempts/handlers/worktree.rs"
  - "crates/server/tests/stream_auth.rs"
  - "crates/remote/src/routes/nodes.rs"
  - "frontend/src/hooks/useNodeLogStream.ts"
  - "frontend/src/hooks/useNodeLogStream.test.ts"
  - "frontend/src/hooks/useAvailableNodes.test.ts"
  - "frontend/src/components/tasks/TaskDetails/ProcessLogsViewer.tsx"
siblings: ["crates/server/src/routes/events.rs","crates/server/tests/events.rs","crates/server/src/routes/terminal.rs","crates/server/tests/harness_smoke.rs","frontend/src/hooks/useDiffStream.ts","crates/remote/src/routes/tasks.rs","crates/services/src/services/connection_token.rs","crates/remote/src/db/node_execution_processes.rs","crates/remote/src/db/node_task_attempts.rs","crates/server/tests/mcp_context_test.rs","frontend/src/hooks/index.ts"]
irreversible: false
scope_test: "crates/server/tests/stream_auth.rs"
allowed_change: mixed
covers_criteria: ["SC2"]
covers_tests: []
---
## Failing test (write first)
File: `crates/server/tests/stream_auth.rs` — create. Use the real `ws_probe()` and `sse_probe()` from task 006. The COMPLETE production census has ten protected stream registrations:

| # | production path | kind | registration | accepted credential class |
|---|---|---|---|---|
| 1 | `/api/events` | SSE | `routes/events.rs:213` | browser session only |
| 2 | `/api/tasks/stream/ws` | WS | `routes/tasks/mod.rs:66` | browser session only |
| 3 | `/api/drafts/stream/ws` | WS | `routes/drafts.rs:53` | browser session only |
| 4 | `/api/task-attempts/{id}/diff/ws` | WS | `routes/task_attempts/mod.rs:100` | browser session OR scoped `connection` token |
| 5 | `/api/task-attempts/by-task-id/{task_id}/diff/ws` | WS | `routes/task_attempts/mod.rs:172` | browser session only |
| 6 | `/api/execution-processes/stream/ws` | WS | `routes/execution_processes.rs:287` | browser session only |
| 7 | `/api/execution-processes/{id}/raw-logs/ws` | WS | `routes/execution_processes.rs:279` | browser session OR scoped `connection` token |
| 8 | `/api/execution-processes/{id}/normalized-logs/ws` | WS | `routes/execution_processes.rs:280` | browser session only |
| 9 | `/api/logs/{execution_id}/live` | WS | `routes/logs.rs:269` | browser session OR scoped `connection` token |
| 10 | `/api/terminal/ws/{session_id}` | WS | `routes/terminal.rs:395` | browser session only |

The route-4/route-5 distinction is grounded in production use, not their similar names. `frontend/src/hooks/useDiffStream.ts:113-139` constructs only `wss://{node}/api/task-attempts/{attempt_id}/diff/ws?token=...`. No frontend or server proxy client constructs the by-task-id diff URL. Route 5 is retained for local browser compatibility but receives neither token alternative.

Census note (verified live against the served router at task-013 dispatch): an authorized browser hitting `/api/task-attempts/{id}/diff/ws` with a nonexistent attempt id observes **500**, not 404 — `load_task_attempt_middleware` does not reject unknown attempts for GET; it runs the Hive fallback and, when that misses, inserts `RemoteAttemptNeeded` and calls the handler (`crates/server/src/middleware/model_loaders.rs:637-641`), whose required `Extension<TaskAttempt>` then rejects with `MissingExtension` → 500 (`crates/server/src/routes/task_attempts/handlers/worktree.rs:46`). This predates the workstream; task 013 preserves the loader unchanged, so the post-change post-auth status stays 500. The census pins it at 500 deliberately: if a later task gives this endpoint 404 semantics, the census test must flag the behavior change. The security boundary under test (anonymous/token-class credentials get 401 BEFORE the loader runs) is unaffected.

```rust
mod common;

fn protected_ws(id: uuid::Uuid) -> Vec<(String, u16)> {
    vec![
        (format!("/api/tasks/stream/ws?project_id={id}"), 101),
        (format!("/api/drafts/stream/ws?project_id={id}"), 101),
        (format!("/api/task-attempts/{id}/diff/ws"), 500), // RemoteAttemptNeeded + required Extension<TaskAttempt> -> MissingExtension (see census note)
        (format!("/api/task-attempts/by-task-id/{id}/diff/ws"), 404),
        (format!("/api/execution-processes/stream/ws?task_attempt_id={id}"), 101),
        (format!("/api/execution-processes/{id}/raw-logs/ws"), 404),
        (format!("/api/execution-processes/{id}/normalized-logs/ws"), 404),
        (format!("/api/logs/{id}/live"), 404),
        (format!("/api/terminal/ws/{id}"), 400),
    ]
}

fn direct_connection_ws(id: uuid::Uuid) -> [String; 3] {
    [format!("/api/task-attempts/{id}/diff/ws"),
     format!("/api/execution-processes/{id}/raw-logs/ws"),
     format!("/api/logs/{id}/live")]
}

fn with_token(path: &str, token: &str) -> String {
    format!("{path}{}token={token}", if path.contains('?') { "&" } else { "?" })
}

#[tokio::test]
#[serial_test::serial]
async fn every_protected_stream_rejects_anonymously_before_lookup_or_upgrade() {
    let h = common::HiveHarness::configured().await;
    let id = uuid::Uuid::new_v4();
    for (path, _) in protected_ws(id) {
        let res = h.ws_probe(&path, None).await;
        assert_eq!(res.status, 401,
            "{path}: anonymous must be 401 (404 means lookup ran; 101 means upgrade ran)");
        assert!(!res.upgraded, "{path}: upgraded anonymously");
    }
    let sse = h.sse_probe("/api/events", None).await;
    assert_eq!(sse.status, 401);
    assert_ne!(sse.content_type.as_deref(), Some("text/event-stream"));
}

#[tokio::test]
#[serial_test::serial]
async fn an_authorized_browser_reaches_every_protected_stream() {
    let h = common::HiveHarness::configured().await;
    let jar = h.authorized_jar().await;
    let id = uuid::Uuid::new_v4();
    for (path, expected) in protected_ws(id) {
        let res = h.ws_probe(&path, Some(&jar)).await;
        assert_eq!(res.status, expected, "{path}: browser boundary result");
    }
    let sse = h.sse_probe("/api/events", Some(&jar)).await;
    assert_eq!(sse.status, 200);
    assert_eq!(sse.content_type.as_deref(), Some("text/event-stream"));
}

#[tokio::test]
#[serial_test::serial]
async fn browser_session_wins_over_an_irrelevant_bad_token_on_direct_streams() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let jar = h.authorized_jar().await;
    let id = uuid::Uuid::new_v4();
    let proxy = mint_proxy_token(SECRET, node_id);

    for path in direct_connection_ws(id) {
        let browser_only = h.ws_probe(&path, Some(&jar)).await;
        assert_ne!(browser_only.status, 401, "{path}: browser session must pass auth");
        for bad_token in ["garbage", proxy.as_str()] {
            let with_bad_token = h.ws_probe(&with_token(&path, bad_token), Some(&jar)).await;
            assert_eq!(with_bad_token.status, browser_only.status,
                "{path}: a valid browser is the chosen OR branch; an irrelevant malformed or wrong-audience query token must not turn it into browser AND token");
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn direct_logs_and_direct_diff_accept_only_a_scoped_connection_token() {
    let _guard = with_connection_secret(SECRET);
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let scoped = mint_connection_token(SECRET, node_id, Some(id));
    let wrong_scope = mint_connection_token(SECRET, node_id, Some(uuid::Uuid::new_v4()));
    let unscoped = mint_connection_token(SECRET, node_id, None);
    let wrong_node = mint_connection_token(SECRET, uuid::Uuid::new_v4(), Some(id));
    let proxy = mint_proxy_token(SECRET, node_id);

    for path in direct_connection_ws(id) {
        assert_eq!(h.ws_probe(&path, None).await.status, 401, "{path}: missing");
        assert_eq!(h.ws_probe(&with_token(&path, "garbage"), None).await.status, 401,
            "{path}: malformed token must stop before lookup");
        assert_eq!(h.ws_probe(&with_token(&path, &wrong_scope), None).await.status, 401,
            "{path}: wrong resource scope must stop before lookup");
        assert_eq!(h.ws_probe(&with_token(&path, &unscoped), None).await.status, 401,
            "{path}: absent resource scope must stop before lookup");
        assert_eq!(h.ws_probe(&with_token(&path, &wrong_node), None).await.status, 401,
            "{path}: wrong target node must stop before lookup");
        assert_eq!(h.ws_probe(&with_token(&path, &proxy), None).await.status, 401,
            "{path}: node_proxy must never open direct logs or diff");
        let accepted = h.ws_probe(&with_token(&path, &scoped), None).await;
        assert_ne!(accepted.status, 401,
            "{path}: correctly scoped connection token must pass auth; body status {}",
            accepted.status);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn browser_only_streams_reject_both_non_browser_token_classes() {
    let node_id = uuid::Uuid::new_v4();
    let h = common::HiveHarness::configured_with_node_auth(SECRET, node_id).await;
    let id = uuid::Uuid::new_v4();
    let conn = mint_connection_token(SECRET, node_id, Some(id));
    let proxy = mint_proxy_token(SECRET, node_id);
    let direct = direct_connection_ws(id);
    for (path, _) in protected_ws(id) {
        if direct.contains(&path) { continue; }
        assert_eq!(h.ws_probe(&with_token(&path, &conn), None).await.status, 401,
            "{path}: connection query token is not an alternative here");
        assert_eq!(h.ws_probe(&with_token(&path, &proxy), None).await.status, 401,
            "{path}: proxy query token is not an alternative here");
    }
}
```
Define test-local `with_connection_secret`, `mint_connection_token` and `mint_proxy_token` exactly from the claim sets in `services/src/services/connection_token.rs`; do not put them in the shared harness. The valid direct-token assertion is deliberately `!= 401`: the random resource may produce 404 or another route-specific response after authentication, but a missing/malformed/wrong-audience/wrong-scope credential must be exactly 401 before that lookup.

Also create `frontend/src/hooks/useNodeLogStream.test.ts`. Mock connection-info and WebSocket, call
`useNodeLogStream(assignmentId, executionProcessId)`, and assert BOTH the Hive request
`/v1/nodes/assignments/{assignmentId}/connection-info?execution_process_id={executionProcessId}`
and direct URL `/api/execution-processes/{executionProcessId}/raw-logs/ws?token=...`. Assert neither
URL substitutes assignment ID or attempt ID for the process ID.

Append pure unit tests beside `get_connection_info` in `crates/remote/src/routes/nodes.rs` for a new private relationship predicate. Feed assignment ID/node ID, process ID/node ID/attempt ID, and attempt ID/assignment ID as UUID values; assert the exact matching tuple passes and wrong assignment, node, process-attempt, or missing attempt-assignment links fail. Add a token-construction unit test that decodes the generated JWT and asserts node_id plus `execution_process_id == Some(process_id)`. These tests require no PostgreSQL service; the handler must call the tested predicate after loading the production repository records and before minting.


## Change
**File:** `crates/server/src/routes/logs.rs`
**Anchor:** `pub fn router` (L265-271). Move only `/logs/{execution_id}/live` out of `router()` into:
```rust
/// Direct live logs: browser session OR Hive `connection` token, never `node_proxy`.
pub fn direct_router() -> Router<DeploymentImpl> {
    Router::new().route("/logs/{execution_id}/live", get(stream_live_logs_ws))
}
```
Add `browser_session: Option<Extension<BrowserSessionCtx>>` to the handler. Only when that extension is absent, require the query token and replace the loose `validate_for_execution` call with `validate_for_resource(token, current_node_id, execution_id)`; obtain current_node_id from NodeRunnerContext and return 401 when absent. When BrowserSessionCtx is present, do not decode an irrelevant query token: D7 is browser OR connection, not browser AND token-if-present. The outer route-class middleware rejects missing/wrong-class/wrong-resource token-only credentials before lookup; the handler retains endpoint-local defense on the token branch before upgrade.

**File:** `crates/server/src/routes/execution_processes.rs`
**Anchor:** `task_attempt_id_router` at L274-288. Remove only `.route("/raw-logs/ws", get(stream_raw_logs_ws))` from that router; leave normalized logs there. Re-register raw logs with the SAME loader:
```rust
/// Direct raw logs: browser session OR scoped Hive `connection` token.
pub fn direct_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let one = Router::new()
        .route("/raw-logs/ws", get(stream_raw_logs_ws))
        .layer(from_fn_with_state(deployment.clone(), load_execution_process_middleware));
    Router::new().nest("/execution-processes/{id}", one)
}
```
Add `browser_session: Option<Extension<BrowserSessionCtx>>`. When absent, require the query token, replace `stream_raw_logs_ws`'s loose check with `validate_for_resource(token, current_node_id, exec_id)`, and fail 401 when node identity is unavailable. When present, skip query-token decoding so a browser session remains a complete alternative.

**File:** `crates/server/src/routes/task_attempts/types.rs`
**Anchor:** `DiffStreamQuery` at L21-25.
**Before:**
```rust
pub struct DiffStreamQuery {
    #[serde(default)]
    pub stats_only: bool,
}
```
**After:**
```rust
pub struct DiffStreamQuery {
    #[serde(default)]
    pub stats_only: bool,
    /// Hive connection token used only by the direct attempt-id diff URL.
    pub token: Option<String>,
}
```

**File:** `crates/server/src/routes/task_attempts/handlers/worktree.rs`
**Anchor:** `stream_task_attempt_diff_ws` at L41-56. Add `browser_session: Option<Extension<BrowserSessionCtx>>`, change its return type from bare `impl IntoResponse` to `Result<impl IntoResponse, ApiError>`, wrap the final upgrade in `Ok(...)`, and on the non-browser branch validate the required query token against this node and `task_attempt.id`:
```rust
    if browser_session.is_none() {
        let token = params.token.as_deref().ok_or(ApiError::Unauthorized)?;
        let node_id = deployment.node_runner_context()
            .ok_or(ApiError::Unauthorized)?.node_id().await
            .ok_or(ApiError::Unauthorized)?;
        deployment.connection_token_validator()
            .validate_for_resource(token, node_id, task_attempt.id)
            .map_err(|_| ApiError::Unauthorized)?;
    }
    // Existing expression becomes `Ok(ws.on_upgrade(...))`.
```
This is intentionally redundant with the outer middleware only for token-authenticated requests: the outer check is before `load_task_attempt_middleware`, satisfying the 401-before-lookup requirement; this handler check makes the production diff endpoint itself stop ignoring `?token=` and is the final guard before upgrade. BrowserSessionCtx selects the independent browser branch, so browsers continue normally whether the URL has no token or an irrelevant stale/bad token.

**File:** `crates/server/src/routes/task_attempts/mod.rs`
**Anchor:** `task_attempt_id_router` at L83-131. Remove only its `.route("/diff/ws", ...)` line. Add:
```rust
/// The production direct-diff path used by useDiffStream.ts with ?token=<connection token>.
pub fn direct_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let diff = Router::new()
        .route("/diff/ws", get(stream_task_attempt_diff_ws))
        .layer(from_fn_with_state(deployment.clone(), load_task_attempt_middleware));
    Router::new().nest("/task-attempts/{id}", diff)
}
```
Do NOT move `/task-attempts/by-task-id/{task_id}/diff/ws`: it has no direct-stream caller, receives no connection token in production, and stays browser-session-only. Task 014 removes it from the proxy-token subtree without granting a token alternative.

**File:** `crates/remote/src/routes/nodes.rs`
**Anchor:** `get_connection_info` L1259-1375. Add required query
`ConnectionInfoQuery { execution_process_id: Uuid }`. Before minting, load that
`NodeExecutionProcess`, load its `NodeTaskAttempt`, and require process.node_id == assignment.node_id
AND attempt.assignment_id == Some(assignment_id); express that comparison through a private pure `connection_resource_matches(...)` predicate and return 404 on mismatch. Generate the connection
token with `Some(query.execution_process_id)`, never `assignment.local_attempt_id`. This converts
the existing broken assignment/attempt/process identifier mix into one exact resource scope.

**File:** `frontend/src/hooks/useNodeLogStream.ts`
Change the hook to `useNodeLogStream(assignmentId, executionProcessId)` — the second parameter is a required argument typed `string | undefined` (not optional with `?`); the remote stream is attempted only when BOTH are defined. Add the encoded process ID
to the connection-info query and construct the direct node URL with `executionProcessId`, not
`assignmentId`. Relay remains assignment-scoped and unchanged.

**File:** `frontend/src/hooks/useAvailableNodes.test.ts`
This file embeds a `useNodeLogStream on a node with no hive` describe block whose two `renderHook`
fixtures call the hook with the pre-013 single-ID signature. The locked required
`execution_process_id` contract makes single-ID fetching unreachable (the hook never fetches
without both IDs), so the "still surfaces a real failure (500)" test fails vacuously. Update ONLY
the two fixtures to `useNodeLogStream('assignment-1', 'process-1')` — every assertion stays
byte-identical; the tests' discriminating purpose (hive-absent swallow vs real-failure surfacing)
is preserved and once again exercises the fetch path.

**File:** `frontend/src/components/tasks/TaskDetails/ProcessLogsViewer.tsx`
Pass the already-available `processId` through `NodeProcessLogsViewer` into `useNodeLogStream`.
`ProcessesTab.tsx` already supplies both `selectedProcess.id` and `attempt.hive_assignment_id`, so it
needs no change.

**File:** `frontend/src/hooks/useNodeLogStream.test.ts` — create with the URL assertions from the
failing-test section. This is a compatibility repair required to preserve production direct raw
logs; do not change diff identifiers, relay URLs, or local log behavior.

**File:** `crates/server/src/routes/mod.rs`
**Anchor:** the four-group construction from task 008. Build the connection group from exactly three direct routers:
```rust
    let connection_stream_routes = Router::new()
        .merge(logs::direct_router())
        .merge(execution_processes::direct_router(&deployment))
        .merge(task_attempts::direct_router(&deployment))
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::auth::node_token::require_session_or_connection_token,
        ));

    let base_routes = public_routes
        .merge(protected_routes)
        .merge(connection_stream_routes)
        .fallback(api_not_found)
        .with_state(deployment);
```
The middleware must be OUTSIDE each direct router's resource loader so missing, malformed, wrong-audience and wrong-resource credentials return 401 before lookup or upgrade.

**Production-path evidence.** Read `frontend/src/hooks/useDiffStream.ts:111-175`: local diff uses `/task-attempts/{attemptId}/diff/ws` with the browser cookie, and remote direct diff uses the same attempt-id route with `connectionInfo.connection_token`. Read `remote/src/routes/tasks.rs:816-852`: Hive passes `assignment.local_attempt_id` into the token's optional resource claim and returns the same attempt ID in `TaskStreamConnectionInfoResponse`. No production code constructs the by-task-id diff URL. This is preserved capability, not new proxy-token stream access.

**Sibling alignment (rubric 9).** Read `routes/events.rs:1-60` for pre-stream HTTP failure behavior, `routes/terminal.rs:384-397` for the stateful browser-only router, `tests/events.rs` for live-stream conventions, and `useDiffStream.ts` plus `remote/src/routes/tasks.rs` for the direct-diff token contract. List every exclusion and guard they make; justify any divergence in the ledger.

**Symbol grounding:** This task introduces `direct_router()` in `routes/logs.rs`, `routes/execution_processes.rs` and `routes/task_attempts/mod.rs`, plus `ConnectionInfoQuery`, private `connection_resource_matches()`, the `connection_stream_routes` group and test-local `direct_connection_ws()`, `with_token()`, `mint_connection_token()` and `mint_proxy_token()`. It calls `require_session_or_connection_token()`, defined by task 007, and never calls `require_session_or_proxy_token()`. `ws_probe()` and `sse_probe()` are defined by task 006. `validate_for_resource()` is defined by task 007 and is called before lookup by middleware and before each direct handler upgrade.

**Required cross-node identifier contract remains locked.** `ConnectionInfoQuery.execution_process_id` is REQUIRED at Hive, the frontend must send it, Hive verifies process/node/attempt/assignment and signs that exact process ID, and the node URL uses that ID. Do not add an optional or legacy fallback and do not split this work from task 013.



## Allowed moves
[
  "Move exactly three registrations into connection_stream_routes: live logs, raw logs and the attempt-id direct diff WebSocket; preserve each existing loader.",
  "Add DiffStreamQuery.token and make stream_task_attempt_diff_ws validate it against task_attempt.id before upgrade.",
  "Keep /task-attempts/by-task-id/{task_id}/diff/ws browser-session-only; do not give it either token class.",
  "Create stream_auth.rs with the complete ten-route protocol census, browser positive cases, three direct connection-token positives, and missing/malformed/wrong-scope/cross-audience negatives.",
  "Repair only the existing raw-log identifier contract across Hive issuance and frontend URL construction; do not change stream bodies, keep-alive, relay behavior, or unrelated routes.",
  "Change the diff handler signature to Result<impl IntoResponse, ApiError> and wrap the upgrade in Ok so strict validation can use ?."
]


## STOP triggers
[
  "Moving `/task-attempts/by-task-id/{task_id}/diff/ws` into connection_stream_routes — no production direct caller sends it a connection token.",
  "Leaving `/api/task-attempts/{id}/diff/ws` browser-only or leaving DiffStreamQuery without token — useDiffStream.ts proves that would break existing cross-node direct diff streaming.",
  "Using require_session_or_proxy_token on any direct log or direct diff route, or accepting Authorization bearer proxy credentials there.",
  "Deleting endpoint-local validate_for_resource on the token branch, OR running it on the BrowserSessionCtx branch — handlers retain binding defense for token-authenticated requests without changing browser OR token into browser AND token-if-present.",
  "Any missing, malformed, wrong-audience or wrong-resource credential reaching a loader or returning 404/101 instead of 401.",
  "Emitting an SSE authentication failure as an event frame instead of HTTP 401.",
  "The census not matching a fresh router grep — add any newly landed stream to the table and protocol tests before proceeding.",
  "Editing projects/mod.rs or model_loaders.rs — proxy routing and its browser-session compatibility are task 014.",
  "Minting the raw-log token from assignment.local_attempt_id or building its node URL from assignmentId — both are the pre-existing broken three-ID contract.",
  "A valid browser session returning 401 merely because a malformed, wrong-audience, wrong-node or wrong-resource query token is also present — BrowserSessionCtx is a complete authorization alternative.",
  "Making execution_process_id optional, defaulting to assignment/local-attempt IDs, adding a compatibility fallback, or splitting this coordinated Hive/node/frontend repair into another task."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test stream_auth" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 013` exits 0.
2. `cargo test -p server` is green, including the repointed SSE tests.
3. Re-run `git grep -n '\.route(' crates/server/src/routes/ | grep -E 'ws|stream|live'` plus `git grep -n 'Sse<' crates/server/src`; record all ten routes and their exact credential class in the ledger.
4. `git grep -n 'connection_token_validator' crates/server/src/routes/` shows exactly the three strict direct handlers: live logs, raw logs and task-attempt direct diff.
5. Record `useDiffStream.ts:113-139` and `remote/src/routes/tasks.rs:816-852` as evidence that only the attempt-id diff route receives the connection token and that its token resource claim is the same local attempt ID.
6. Paste the protocol assertions showing 401 for missing, malformed, wrong-scope and proxy-audience tokens before a random resource can return its non-auth status.
7. `cargo test -p remote connection_resource_matches` and `(cd frontend && npx vitest run src/hooks/useNodeLogStream.test.ts)` prove the emitted raw-log token and production direct URL use the same execution-process ID.
8. Valid positive URLs include required project_id/task_attempt_id query fields; nonexistent terminal expects 400; token appending uses `&` when a query already exists.
9. Paste the three-route regression assertions proving a valid browser gets the same post-auth status with no token, malformed token, or proxy-audience token; this is the D7 OR-semantics guard.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 013` exits 0
