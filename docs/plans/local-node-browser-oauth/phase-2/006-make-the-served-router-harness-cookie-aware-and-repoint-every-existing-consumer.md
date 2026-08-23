---
id: "006"
phase: 2
title: "Make the served-router harness cookie-aware and repoint every existing consumer"
status: ready
depends_on: ["001","002","005"]
parallel: false
conflicts_with: ["002","008"]
files:
  - "Cargo.lock"
  - "crates/server/Cargo.toml"
  - "crates/server/tests/common/mod.rs"
  - "crates/server/tests/harness_smoke.rs"
  - "crates/server/tests/events.rs"
  - "crates/server/tests/nodes_routes.rs"
  - "crates/server/tests/projects_with_stats.rs"
  - "crates/server/tests/swarm_labels_routes.rs"
  - "crates/server/tests/swarm_projects_routes.rs"
  - "crates/server/tests/swarm_templates_routes.rs"
  - "crates/server/tests/tasks_delete_routes.rs"
siblings: ["crates/services/Cargo.toml"]
irreversible: false
scope_test: "crates/server/tests/harness_smoke.rs"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `crates/server/tests/harness_smoke.rs` — append eight focused harness tests. Every test in this directory is `#[serial_test::serial]` because `HiveHarness` mutates process env (`VK_ASSET_DIR`, `VK_DATABASE_PATH`, `VK_SHARED_API_BASE`); omitting it causes DB and hive-config bleed.

```rust
#[tokio::test]
#[serial_test::serial]
async fn jars_are_independent_and_capture_set_cookie() {
    let h = common::HiveHarness::configured().await;
    let mut a = common::CookieJar::new();
    let b = common::CookieJar::new();
    a.insert("vks_probe", "A");
    assert_eq!(a.header_value().as_deref(), Some("vks_probe=A"));
    assert_eq!(b.header_value(), None, "jars must not share state");

    let res = h.get_with("/api/health", &mut a).await;
    res.assert_registered();
    assert_eq!(res.status, 200);
    assert!(res.set_cookie.is_empty(), "health sets no cookie: {:?}", res.set_cookie);

    // Attribute names are case-insensitive and values are parsed as complete integers.
    a.apply(&["vks_probe=gone; max-age=0".to_string()]);
    assert_eq!(a.get("vks_probe"), None);
    a.apply(&["vks_probe=kept; Max-Age=01".to_string()]);
    assert_eq!(a.get("vks_probe"), Some("kept"));
}

#[tokio::test]
#[serial_test::serial]
async fn hive_oauth_mocks_hand_out_successive_handoff_ids() {
    let h = common::HiveHarness::configured().await;
    let sub = uuid::Uuid::new_v4();
    let first = h.mock_hive_oauth("code-1", "acc-1", "ref-1", sub).await;
    let second = h.mock_hive_oauth("code-2", "acc-2", "ref-2", sub).await;
    assert_ne!(first, second);
    for (m, p) in [("POST", "/v1/oauth/web/init"), ("POST", "/v1/oauth/web/redeem"),
                   ("GET", "/v1/profile")] {
        assert!(h.hive_mock_registered(m, p).await, "missing mock for {m} {p}");
    }
    // Two successive initiations must receive the two DIFFERENT ids, in order. Without
    // `.up_to_n_times(1)` on the init mock, wiremock's first-match-wins resolution would return
    // `first` twice and every two-login test would fail for an unrelated reason.
    let mut jar = common::CookieJar::new();
    let r1 = h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut jar).await;
    let r2 = h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), &mut jar).await;
    assert!(r1.body.contains(&first.to_string()), "body: {}", r1.body);
    assert!(r2.body.contains(&second.to_string()), "body: {}", r2.body);
}

#[tokio::test]
#[serial_test::serial]
async fn probes_speak_the_real_protocols() {
    let h = common::HiveHarness::configured().await;
    let jar = h.authorized_jar().await;

    // A REAL websocket handshake against a real WS route completes with 101 and an open socket.
    let ws = h.ws_probe(&format!("/api/tasks/stream/ws?project_id={}", uuid::Uuid::new_v4()), Some(&jar)).await;
    assert_eq!(ws.status, 101, "a valid handshake on a real WS route must upgrade");
    assert!(ws.upgraded, "tokio-tungstenite must report an established connection");

    // A REAL SSE request returns 200 + text/event-stream, and the probe must NOT hang on the
    // endless body.
    let sse = tokio::time::timeout(std::time::Duration::from_secs(5),
        h.sse_probe("/api/events", Some(&jar))).await
        .expect("sse_probe must not consume the endless body");
    assert_eq!(sse.status, 200);
    assert_eq!(sse.content_type.as_deref(), Some("text/event-stream"));
}

#[tokio::test]
#[serial_test::serial]
async fn profile_mocks_are_keyed_by_the_exact_generated_candidate_jwt() {
    let h = common::HiveHarness::configured().await;
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    h.mock_hive_oauth("code-a", "access-a", "refresh-a", first).await;
    h.mock_hive_oauth("code-b", "access-b", "refresh-b", second).await;

    // The access-token argument is a stable LABEL. Every path derives the same complete JWT.
    let jwt_a = h.access_token_for_label("access-a");
    let jwt_b = h.access_token_for_label("access-b");
    assert_ne!(jwt_a, "access-a");
    assert_ne!(jwt_a, jwt_b);
    assert!(utils::jwt::extract_expiration(&jwt_a).unwrap() > chrono::Utc::now());
    assert_eq!(h.redeemed_access_token("code-a").await, jwt_a,
        "redeem must return the exact JWT used by profile matching");
    assert_eq!(h.profile_subject_for("access-a").await, first);
    assert_eq!(h.profile_subject_for("access-b").await, second);
}

#[tokio::test]
#[serial_test::serial]
async fn restart_reuses_the_same_assets_dir_and_database() {
    let first = common::HiveHarness::configured().await;
    let project_id = first.seed_project("restart-probe", &[]).await;
    let old_generation = first.server_generation();
    // Deliberately overwrite process-global env through a second live harness. restart() must
    // restore FIRST's retained paths/configuration before reconstruction.
    let second = common::HiveHarness::configured().await;
    let second_project_id = second.seed_project("other-harness", &[]).await;
    let h = first.restart().await;
    assert_eq!(h.last_completed_server_generation(), Some(old_generation),
        "restart must record the old generation only after its serve JoinHandle completes");
    assert_eq!(h.server_generation(), old_generation + 1,
        "the replacement server is a new generation over the same persisted state");
    let mut jar = h.authorized_jar().await;
    let res = h.get_with("/api/projects", &mut jar).await;
    res.assert_registered();
    assert!(res.body.contains(&project_id.to_string()),
        "restart must reuse the same sqlite file; body: {}", res.body);
    assert!(!res.body.contains(&second_project_id.to_string()),
        "restart must not switch to another live harness's sqlite file; body: {}", res.body);
}
```

For the seven repointed consumer files: **N/A — covered by existing tests** (`crates/server/tests/events.rs`, `nodes_routes.rs`, `projects_with_stats.rs`, `swarm_labels_routes.rs`, `swarm_projects_routes.rs`, `swarm_templates_routes.rs`, `tasks_delete_routes.rs`). The repoint is mechanical and the gate relies on those staying green.

Additional executable harness tests appended in the same file:

```rust
#[tokio::test]
#[serial_test::serial]
async fn resp_preserves_all_repeated_headers() {
    let h = common::HiveHarness::configured().await;
    h.mock_redirect("/header-probe", "/target", &["probe=a; HttpOnly", "other=b"]).await;
    let mut jar = common::CookieJar::new();
    let res = h.get_no_redirect("/header-probe", &mut jar).await;
    assert_eq!(res.status, 302);
    assert_eq!(res.headers.get_all(reqwest::header::SET_COOKIE).iter().count(), 2);
    assert_eq!(res.set_cookie.len(), 2);
}

#[tokio::test]
#[serial_test::serial]
async fn no_redirect_preserves_location() {
    let h = common::HiveHarness::configured().await;
    h.mock_redirect("/location-probe", "/target", &[]).await;
    let mut jar = common::CookieJar::new();
    let res = h.get_no_redirect("/location-probe", &mut jar).await;
    assert_eq!(res.status, 302);
    assert_eq!(res.location(), Some("/target"));
}

#[tokio::test]
#[serial_test::serial]
async fn priority_one_outage_overrides_signal_and_record_the_exact_request() {
    let h = common::HiveHarness::configured().await;
    let owner = uuid::Uuid::new_v4();
    h.mock_hive_oauth("code-a", "access-a", "refresh-a", owner).await;
    let reached = h.mock_hive_failure("POST", "/v1/tokens/refresh", 503).await;
    // Force refresh-only persisted credentials, then drive RemoteClient::access_token(). This is
    // the real production refresh path, not a raw reqwest request to the mock URL.
    h.write_refresh_only_credentials("test-refresh-token").await;
    let request = spawn_real_refresh_request(&h);
    tokio::time::timeout(std::time::Duration::from_secs(2), reached)
        .await.expect("refresh never reached Wiremock").unwrap();
    assert_eq!(h.hive_request_count("POST", "/v1/tokens/refresh").await, 1);
    request.abort();
    let _ = request.await;
}
```

The connection-reset and delayed responders receive equivalent self-tests: await the responder's request-arrival signal under a 2-second diagnostic watchdog, assert exact method/path count from `MockServer::received_requests()`, then abort/await the caller. No test sleeps through RemoteClient retry/backoff.



## Change
**File:** `crates/server/Cargo.toml`
**Anchor:** the `[dev-dependencies]` block (verified: `db`, `serial_test`, `wiremock`, `tempfile`, `jsonwebtoken`, `tracing-test`).
**Before:**
```toml
[dev-dependencies]
db = { path = "../db", features = ["test-utils"] }
serial_test = "3.0"
```
**After:**
```toml
[dev-dependencies]
db = { path = "../db", features = ["test-utils"] }
serial_test = "3.0"
# Real WebSocket client for the TS3 protocol tests. An ordinary GET carrying hand-written or
# deliberately malformed upgrade headers is NOT a protocol test: it cannot observe a 101, so it cannot
# distinguish "authentication rejected the request" from "the upgrade was refused". Version 0.28
# matches crates/services and crates/utils, so the workspace resolves one copy.
tokio-tungstenite = { version = "0.28", features = ["rustls-tls-webpki-roots"] }
```
Add nothing else to this file; the `base64` runtime dependency belongs to task 002.

**File:** `crates/server/tests/common/mod.rs`

**Anchor 1 — `pub struct Resp` (L24-30).**
**Before:**
```rust
pub struct Resp {
    pub status: u16,
    pub body: String,
    #[allow(dead_code)]
    pub content_type: Option<String>,
}
```
**After:** the same struct plus one field, with every constructor updated to fill it:
```rust
pub struct Resp {
    pub status: u16,
    pub body: String,
    #[allow(dead_code)]
    pub content_type: Option<String>,
    /// Every response header, cloned BEFORE `.text()`/body consumption. HeaderMap preserves
    /// repeated values, including every Location/Set-Cookie surface task 018 must scan.
    #[allow(dead_code)]
    pub headers: reqwest::header::HeaderMap,
    /// RAW `Set-Cookie` lines, verbatim and unparsed, derived from `headers.get_all(SET_COOKIE)`.
    #[allow(dead_code)]
    pub set_cookie: Vec<String>,
}
```
In `get`, `post`, `delete`, every jar-aware driver, and `get_no_redirect`, clone `res.headers()` BEFORE consuming the body; derive `content_type` and `set_cookie` from that clone. Add `Resp::location() -> Option<&str>` using `headers.get(LOCATION)`. `get_no_redirect` uses a reqwest client with `redirect::Policy::none()` so Location is observable rather than followed.

**Anchor 2 — after the `impl Resp { ... }` block, before `#[allow(dead_code)] impl HiveHarness`.**
**After:** add the jar type and the probe result type. This task delivers the WHOLE harness API the later tasks need, so no later task edits this file:
```rust
/// An independent browser cookie jar. Two jars in one test are two clean browsers.
#[allow(dead_code)]
#[derive(Default)]
pub struct CookieJar {
    cookies: std::collections::BTreeMap<String, String>,
}

#[allow(dead_code)]
impl CookieJar {
    pub fn new() -> Self { Self::default() }
    /// Set a cookie directly (used to forge a wrong-browser value).
    pub fn insert(&mut self, name: &str, value: &str);
    pub fn get(&self, name: &str) -> Option<&str>;
    /// The `Cookie:` request-header value, or None when the jar is empty.
    pub fn header_value(&self) -> Option<String>;
    /// Apply raw `Set-Cookie` lines: store `name=value`, and REMOVE the cookie when a
    /// semicolon-delimited, case-insensitive `Max-Age` attribute parses as integer zero. Complete
    /// numeric parsing is required: `Max-Age=01` is one second and must not be treated as zero.
    pub fn apply(&mut self, set_cookie: &[String]);
    /// A jar that shares nothing with `self` -- an explicitly clean second browser.
    pub fn fresh() -> Self { Self::default() }
}

/// Outcome of a REAL protocol probe (websocket handshake or SSE request).
#[allow(dead_code)]
pub struct ProtocolProbe {
    /// 101 on a completed upgrade; otherwise the HTTP status of the rejection.
    pub status: u16,
    /// True only when a websocket connection was actually established.
    pub upgraded: bool,
    pub content_type: Option<String>,
}
```

**Anchor 3 — inside `#[allow(dead_code)] impl HiveHarness`, after the existing `delete` method.**
**After:** add the jar-aware drivers, the two protocol probes, the hive mocks and restart:
```rust
/// GET through a jar: sends the jar's `Cookie:` header and applies the response's Set-Cookie.
pub async fn get_with(&self, path: &str, jar: &mut CookieJar) -> Resp;
pub async fn post_with(&self, path: &str, body: serde_json::Value, jar: &mut CookieJar) -> Resp;
pub async fn delete_with(&self, path: &str, jar: &mut CookieJar) -> Resp;
/// GET with arbitrary extra headers and NO jar (anonymous or hand-built requests).
pub async fn get_with_headers(&self, path: &str, headers: &[(&str, &str)]) -> Resp;
/// GET that does NOT follow redirects, so a callback's `Location` can be inspected.
pub async fn get_no_redirect(&self, path: &str, jar: &mut CookieJar) -> Resp;
/// Existing accessor retained and explicitly available to downstream sync tests.
pub fn deployment(&self) -> &DeploymentImpl;
/// Exact complete generated JWT for a stable access-token label.
pub fn access_token_for_label(&self, label: &str) -> String;
/// Observe the exact JWT returned by the redeem mock for this app code (harness self-test only).
pub async fn redeemed_access_token(&self, app_code: &str) -> String;

/// REAL websocket handshake via `tokio_tungstenite::connect_async`.
///
/// Builds `ws://{addr}{path}` into a `http::Request` (IntoClientRequest supplies the correct
/// `Upgrade: websocket`, `Connection: Upgrade`, `Sec-WebSocket-Version: 13` and a freshly
/// generated `Sec-WebSocket-Key`), attaches the jar's `Cookie:` header, and connects.
///   * success  -> ProtocolProbe { status: 101, upgraded: true }; the socket is closed
///                 immediately so nothing hangs.
///   * rejection-> `tungstenite::Error::Http(resp)` carries the real status; return it with
///                 upgraded: false. THIS is what makes 401-before-upgrade observable.
/// `?token=` and other query parameters are part of `path` and are forwarded verbatim.
pub async fn ws_probe(&self, path: &str, jar: Option<&CookieJar>) -> ProtocolProbe;

/// The same real handshake with explicit request headers, used for cross-class refusal tests.
/// `ws_probe()` delegates here with an empty header slice. Header names/values are attached to
/// the tungstenite client request after `IntoClientRequest` creates the valid upgrade headers.
pub async fn ws_probe_with_headers(&self, path: &str, jar: Option<&CookieJar>,
    headers: &[(&str, &str)]) -> ProtocolProbe;

/// REAL SSE request: GET with `Accept: text/event-stream`.
///
/// Returns status + content-type and DROPS the response without reading the body -- an
/// authorized SSE route answers 200 with a stream that never ends, and reading it to completion
/// would hang the test rather than fail it.
pub async fn sse_probe(&self, path: &str, jar: Option<&CookieJar>) -> ProtocolProbe;

/// Mount the three hive endpoints a browser login needs: `/v1/oauth/web/init` (returns a fresh
/// handoff_id + authorize_url), `/v1/oauth/web/redeem` (returns the given tokens for the given
/// app_code) and `/v1/profile` (ProfileResponse with `user_id = subject`). Returns the
/// handoff_id the init mock will hand out.
///
/// The init mock MUST be mounted with `.up_to_n_times(1)`. `up_to_n_times()` is an EXISTING
/// wiremock 0.6 builder method (not introduced here) -- verified in the vendored dependency
/// source, where `MountedMockSet::handle_request` stable-sorts on `specification.priority` (all
/// default to 5), skips mocks in state `OutOfScope`, and takes the FIRST remaining match
/// (`mock_set.rs:57-72`). Without the limit a second `mock_hive_oauth` call on the same harness
/// is never reached and BOTH logins receive the FIRST handoff_id, breaking every two-login test
/// (011, 012, 018) for a reason unrelated to the feature. `redeem` is keyed by `app_code` and
/// each `profile` mock MUST additionally match `Authorization: Bearer <access_token>` because different login candidates have different subjects; only `init` needs the one-use limit.
pub async fn mock_hive_oauth(&self, app_code: &str, access_token_label: &str,
    refresh_token: &str, subject: uuid::Uuid) -> uuid::Uuid;
/// Replace the profile mock for `access_token_label` with a priority-1 responder that signals
/// request arrival, then returns the same valid ProfileResponse after `delay`. Task 012 uses this
/// as a deterministic callback-vs-disconnect barrier: the callback has claimed its handoff and is
/// in candidate Hive I/O when disconnect linearizes.
pub async fn delay_hive_profile(&self, access_token_label: &str, subject: uuid::Uuid,
    delay: std::time::Duration) -> tokio::sync::oneshot::Receiver<()>;
/// The access-token argument remains source-compatible but is a stable LABEL. Derive (or memoize)
/// a deterministic HS256 JWT with a fixed future `exp` and a `test_label` claim. Redeem returns
/// that exact complete JWT; the `/v1/profile` matcher uses `Authorization: Bearer <that JWT>`.
/// `access_token_for_label(label)` and every downstream test must obtain the identical string.
/// A plaintext label is never used as a bearer token.
/// Test-only observation: derives the same JWT from `label`, requests `/v1/profile` with it, and
/// returns the matched subject.
pub async fn profile_subject_for(&self, label: &str) -> uuid::Uuid;
/// Build a validator-enabled node harness and set its NodeRunnerContext node_id before serving.
/// Sets VK_HIVE_URL/VK_NODE_API_KEY/VK_CONNECTION_TOKEN_SECRET before LocalDeployment::new;
/// after construction writes expected_node_id into the public runner state.
pub async fn configured_with_node_auth(secret: &str, expected_node_id: uuid::Uuid) -> Self;
/// True when a mock is mounted for this method+path (used by the harness self-test).
pub async fn hive_mock_registered(&self, method: &str, path: &str) -> bool;
/// Mount a priority-1 exact method+path override returning `status`. The custom Respond
/// implementation sends a one-shot signal synchronously when Wiremock receives the request.
pub async fn mock_hive_failure(&self, method: &str, path: &str, status: u16)
    -> tokio::sync::oneshot::Receiver<()>;
/// Priority-1 exact override whose `RespondErr` signals then returns `std::io::ErrorKind::ConnectionReset`.
pub async fn mock_hive_connection_reset(&self, method: &str, path: &str)
    -> tokio::sync::oneshot::Receiver<()>;
/// Priority-1 exact override whose `Respond` signals then returns a ResponseTemplate with a long
/// delay. Tests abort the caller after observing the signal; they never wait through the delay.
pub async fn mock_hive_delayed(&self, method: &str, path: &str)
    -> tokio::sync::oneshot::Receiver<()>;
/// Count recorded requests matching BOTH exact HTTP method and path.
pub async fn hive_request_count(&self, method: &str, path: &str) -> usize;

/// Rebuild the deployment and served router on the SAME temp dir. HiveHarness must store an
/// `Option<oneshot::Sender<()>>` plus the `JoinHandle` returned by spawning axum::serve with
/// `with_graceful_shutdown`, `server_generation: u64`, and
/// `last_completed_server_generation: Option<u64>`. Signal shutdown and await the old serve
/// JoinHandle. ONLY after that await succeeds, record the old generation as completed; then drop
/// its deployment/router state, rebuild, increment the generation, and bind the replacement
/// listener. Immediately before `LocalDeployment::new()`, restore this harness's retained
/// `VK_ASSET_DIR`, `VK_DATABASE_PATH`, `VK_SHARED_API_BASE`, and node-auth env state: another live
/// harness may have overwritten process globals since construction. The OS may reuse the same
/// ephemeral port, so socket-address inequality is never a
/// lifecycle assertion. The wiremock server and temp directory are preserved. Merely starting a
/// second deployment is NOT a restart and cannot produce a completed-generation observation.
pub async fn restart(mut self) -> Self;
/// Monotonic test-harness server generation; starts at 1.
pub fn server_generation(&self) -> u64;
/// Set to the old generation only AFTER its axum serve JoinHandle has completed.
pub fn last_completed_server_generation(&self) -> Option<u64>;
/// The raw sqlite pool, for asserting persisted browser-auth state.
pub fn pool(&self) -> &sqlx::SqlitePool { &self.deployment.db().pool }
/// Path of the credentials file inside the harness temp dir (for disconnect assertions).
pub fn credentials_path(&self) -> std::path::PathBuf;
/// Replace this harness's credential file with refresh-token-only credentials so the next
/// `RemoteClient::access_token()` call must traverse the real `/v1/tokens/refresh` path.
pub async fn write_refresh_only_credentials(&self, refresh_token: &str);

/// A jar holding a REAL live browser session, created straight through the db model (never
/// hand-written SQL/DDL). This exists so the seven pre-existing served-router tests can keep
/// driving protected routes once task 008 makes the API deny-by-default, WITHOUT each of them
/// having to run a full OAuth login.
pub async fn authorized_jar(&self) -> CookieJar;
// impl: let raw = OsTokenSource.generate_token();
//       db::models::browser_auth::create_session(self.pool(), Uuid::new_v4(),
//           &server::auth::seams::hash_token(&raw), Uuid::new_v4(), 0).await.unwrap();
//       let mut jar = CookieJar::new();
//       jar.insert("vks_browser_session", &raw);   // literal on purpose: see below
```
Use the LITERAL cookie name `"vks_browser_session"` in the harness so this task depends only on 001 (schema), 002 (`hash_token`) and 005 (`create_session`), never on task 007.

**Anchor 4 — repoint the seven existing consumers.** In each of `events.rs`, `nodes_routes.rs`, `projects_with_stats.rs`, `swarm_labels_routes.rs`, `swarm_projects_routes.rs`, `swarm_templates_routes.rs`, `tasks_delete_routes.rs`:
- **Before:** `let res = h.get("/api/…").await;` (and the `post`/`delete` equivalents; in `events.rs` the SSE calls are raw reqwest builders — `let res = client.get(&url).send().await.unwrap();` at L118, L196, L269, L336, L381).
- **After:** the same call through an authorized jar — `let mut jar = h.authorized_jar().await;` once per test, then `h.get_with("/api/…", &mut jar)` / `h.post_with(...)` / `h.delete_with(...)` as appropriate; for the raw reqwest SSE builders add `.header("cookie", jar.header_value().unwrap())` to the existing builder chain and change nothing else about the stream handling. Constructing an authorized jar without passing it to the request is not a repoint and fails this task.
This repoint is BEHAVIOR-PRESERVING and green immediately: the authorization boundary does not exist yet, so sending a cookie changes nothing. Doing it here — before task 008 — is what keeps `cargo test -p server` green across the whole plan instead of red between two tasks.

**File:** `crates/server/tests/harness_smoke.rs`
**Anchor:** end of file, after the existing `harness_detects_an_unregistered_route` test.
**After:** the eight tests from the Failing-test section, appended verbatim. Its two existing tests that call `/api/organizations` are repointed through `authorized_jar()` exactly like the other consumers.

**Sibling alignment (rubric 9).** Read `crates/server/tests/events.rs` before writing the probes — it is the only existing test that consumes a live stream and documents the axum SSE frame format and why a body read must break early; and read `crates/services/Cargo.toml:52` for the exact `tokio-tungstenite` version/feature string to copy.

**Symbol grounding:** This task introduces the harness methods `get_with()`, `post_with()`, `delete_with()`, `get_with_headers()`, `get_no_redirect()`, `ws_probe()`, `ws_probe_with_headers()`, `sse_probe()`, `mock_hive_oauth()`, `profile_subject_for()`, `configured_with_node_auth()`, `hive_mock_registered()`, `mock_hive_failure()`, `restart()`, `server_generation()`, `last_completed_server_generation()`, `pool()`, `credentials_path()`, `write_refresh_only_credentials()` and `authorized_jar()` on `HiveHarness`, plus the `CookieJar` and `ProtocolProbe` types. `up_to_n_times()` is NOT introduced here: it is an existing wiremock 0.6 `MockBuilder` method, verified in the vendored dependency source alongside `MountedMockSet::handle_request`'s first-match-wins resolution. `hash_token()` is likewise not introduced here — it is defined by task 002 and merely called by `authorized_jar()`.

**OAuth JWT construction (mandatory).** Replace the existing zero-argument `test_access_token()` helper with `test_access_token(label: &str)`. Serialize `{ exp: 4_102_444_800_i64, test_label: label }` and encode deterministically. The exact signature is test-only and unverified by `extract_expiration`; the stable full compact JWT string is the contract. `mock_hive_oauth`, `access_token_for_label`, `redeemed_access_token`, and `profile_subject_for` MUST all call the same derivation/memoization path. The harness self-test calls production `utils::jwt::extract_expiration` and proves the result is future-dated.

**Wiremock outage grounding.** Wiremock 0.6.5 provides `Mock::with_priority(1)`, `MockBuilder::respond_with`, `MockBuilder::respond_with_err`, `Respond`, `RespondErr`, `ResponseTemplate::set_delay`, and `MockServer::received_requests()`. Lower numeric priority wins; every outage override is priority 1 so the default successful refresh/profile/init mock cannot shadow it. Custom responders hold `Mutex<Option<oneshot::Sender<()>>>` (or equivalent one-shot state), signal exactly once from `respond`/`respond_err`, and then return the failure/delay. Do not claim a nonexistent async responder API.

**All 14 `/api/events` builders are structural edits, not a grep oracle.** In `crates/server/tests/events.rs`, enumerate and edit the request builders at current lines **118, 196, 269, 336, 381, 461, 574, 666, 770, 797, 838, 850, 883, 895**. This includes seven one-line `client.get(&url)` forms and seven multiline/inline `reqwest::Client::new().get(...)` / `client.get(format!(...))` forms. In every containing test create one `let jar = h.authorized_jar().await;`, insert `.header(reqwest::header::COOKIE, jar.header_value().unwrap())` into that exact builder before `.send()`, and preserve every existing status, body/frame, cursor, error, teardown, and ordering assertion unchanged. The acceptance check is review of these 14 concrete builder chains and the existing tests passing; raw grep counts and comments do not qualify.


**WAI symbol grounding.** This task owns `access_token_for_label()`, `redeemed_access_token()`, `delay_hive_profile()`, `mock_hive_connection_reset()`, `mock_hive_delayed()`, and `hive_request_count()` through the typed declarations above. It uses Wiremock 0.6.5's dependency-owned `with_priority()` method; it does not reimplement that dependency API.


## Allowed moves
[
  "Add exactly one dev-dependency (tokio-tungstenite) to crates/server/Cargo.toml and record the server package dependency edge in Cargo.lock.",
  "Extend Resp with a cloned all-headers collection, keep raw Set-Cookie values, add location(), and populate headers before every body read.",
  "Add CookieJar, ProtocolProbe, access-token-label JWT derivation, deployment()/response helpers, exact request counting, and priority-1 signalled outage responders to HiveHarness.",
  "Append eight focused harness tests covering jars, successive handoffs, real WS/SSE, exact generated JWT/profile matching, all headers/Location, signalled priority override, and restart generations.",
  "Structurally add the authorized cookie header to all 14 cited `/api/events` request builders and repoint the other six consumer files; preserve every assertion.",
  "Retain and await the old serve JoinHandle, record generation completion only after await, reuse persisted paths, and permit OS port reuse.",
  "Retain constructor environment values and restore this harness's own values immediately before restart reconstruction, even when another live harness overwrote process globals.",
  "Do not change existing configured()/hive_absent()/get()/post()/delete()/seed_* semantics or Resp registration helpers."
]


## STOP triggers
[
  "Any urge to enable reqwest's cookie_store feature — it discards the cookie attributes SC5/D9 require and would silently weaken every later assertion.",
  "Implementing ws_probe as an ordinary GET with hand-written or deliberately malformed upgrade headers — it cannot observe a 101 and therefore cannot distinguish an auth rejection from an upgrade refusal. Use tokio_tungstenite::connect_async.",
  "sse_probe or ws_probe reading the response body — an authorized SSE route returns an endless 200 stream and the test would hang rather than fail.",
  "mock_hive_oauth mounting init without `.up_to_n_times(1)` OR mounting profile without a candidate bearer-token matcher — either makes the second login silently reuse first-login state.",
  "restart() starting a second server without signaling and awaiting the first server's graceful shutdown, or recording completion before the old JoinHandle returns — either is concurrent deployment, not TS4 restart evidence.",
  "Any assertion in the seven repointed files changing value or meaning — that half of the task is mechanical; a changed expectation means you altered behaviour.",
  "authorized_jar() writing a session row with raw SQL or a hand-written CREATE TABLE — it must go through db::models::browser_auth::create_session on the deployment's migrated pool.",
  "Any new test in this directory without #[serial_test::serial].",
  "A later task needing a harness method that is not on this list — STOP: harness changes belong to THIS task, otherwise every downstream task becomes mutually conflicting on common/mod.rs.",
  "Asserting that the replacement address differs from the old ephemeral address, or probing the old address after replacement bind — the OS may correctly reuse the port; use completed server generations.",
  "Treating mock_hive_oauth access-token labels as literal bearer tokens — derive the same complete future-expiring JWT for redeem, profile matching, profile_subject_for and task 018 scanning.",
  "Mounting an outage mock at default priority — it can be shadowed by an earlier success mock; exact method/path overrides are priority 1 and signal on receipt.",
  "Sleeping through RemoteClient retries or a delayed response — await the Wiremock arrival signal under a short diagnostic watchdog, assert exact request observation, then abort/await the caller.",
  "A delayed-profile helper that returns an empty/generic response instead of the same valid subject response as mock_hive_oauth — task 012 must resume the real success path after disconnect.",
  "Using a grep/comment count as proof for events.rs — all 14 cited request-builder expressions must be individually edited while retaining their assertions.",
  "Consuming a response body before cloning all headers, or allowing get_no_redirect to follow Location.",
  "Cookie deletion based on case-sensitive substring matching rather than a complete, case-insensitive Max-Age attribute parse.",
  "An outage self-test that sends raw reqwest directly to `/v1/tokens/refresh` instead of forcing refresh-only credentials and calling the deployment RemoteClient's real access-token path."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test harness_smoke" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 006` exits 0.
2. `cargo test -p server --test harness_smoke` — the 3 pre-existing tests plus all eight protocol/JWT/header/outage/restart tests green.
3. `cargo test -p server` — the ENTIRE server test suite green, including the seven repointed files.
4. `git diff --stat crates/server/tests/` — the seven consumer files show only cookie-plumbing lines; no assertion text changed.
5. `cargo tree -p server --duplicates | grep -i tungstenite` shows a single tokio-tungstenite version.
6. Review the 14 cited `events.rs` builders one by one; each carries the authorized Cookie header and every pre-existing assertion remains.
7. The JWT self-test proves production expiration extraction, exact redeem equality, and profile matching; outage self-tests prove priority-1 method/path observation without retry sleeps.
8. `Resp` header test proves repeated Set-Cookie and Location survive body consumption.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 006` exits 0
