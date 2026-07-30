---
id: "100"
phase: 1
title: "Build the hive-proxy test harness the frozen spec's Test strategy requires"
status: ready
depends_on: ["099"]
parallel: false
conflicts_with: []
files:
  - crates/server/tests/common/mod.rs
  - crates/server/tests/harness_smoke.rs
  - crates/server/Cargo.toml
  - Cargo.lock
siblings:
  - crates/server/tests/mcp_context_test.rs
  - crates/db/src/test_utils.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC1, SC4]
---

## Failing test (write first)

`crates/server/tests/harness_smoke.rs` — the harness proving itself before anything depends on it:

```rust
mod common;

#[tokio::test]
#[serial_test::serial]
async fn harness_serves_a_configured_hive() {
    let h = common::HiveHarness::configured().await;
    h.mock_json("GET", "/v1/organizations", 200, serde_json::json!({"organizations": []}))
        .await;
    let res = h.get("/api/organizations").await;
    assert_eq!(res.status, 200, "body: {}", res.body);
    assert!(res.body.contains("\"success\":true"), "body: {}", res.body);
}

// A 401 here means the credential seeding in `configured()` did not take effect — see
// Amendment B. Do NOT "fix" it by relaxing this assertion to `assert_ne!(res.status, 404)`;
// the 200 is the frozen spec's required SC1/SC4 signal and is the whole reason task 099 exists.

#[tokio::test]
#[serial_test::serial]
async fn harness_serves_an_absent_hive() {
    let h = common::HiveHarness::hive_absent().await;
    let res = h.get("/api/organizations").await;
    assert_ne!(res.status, 404, "route must be registered");
    assert_ne!(res.status, 500, "absent hive is not a server error");
}
```

`/api/organizations` is used deliberately: it is already registered on `main`, so this file goes
RED (harness missing) then GREEN **without** depending on tasks 101-104. If it only passed once
the restored routes existed, it would be testing them rather than itself.

## Why this task exists

The frozen spec's `## Test strategy` requires "Per-module route tests for each restored proxy:
hive-configured returns `200` + `success: true` (against a mocked `RemoteClient`), and hive-absent
returns the not-configured variant rather than a 500", plus a `ProjectWithStats` handler test.
Every such test needs a `DeploymentImpl`, and no test in this repo builds one. This task creates
the smallest seam that makes those tests possible; 101-104 and 301 then consume it.

The raw material already exists — verified at decomposition:

- `wiremock = "0.6"` is a dev-dependency of `crates/services`
- `crates/server` already dev-depends on `db` with `features = ["test-utils"]` and on
  `serial_test = "3.0"` (env-var mutation must be serialised)
- `DeploymentImpl` is built by `Deployment::new()`
  (`crates/local-deployment/src/lib.rs:101`), which takes no arguments and reads
  `VK_SHARED_API_BASE` (`lib.rs:188`) — pointing that at a wiremock server is the whole trick
- `VK_DATABASE_PATH` overrides the database location (`crates/utils/src/assets.rs:48-59`,
  consumed by `crates/db/src/lib.rs:474` `create_pool` and `:317` `bootstrap`)

**Prefer the env-var route: it requires NO production-code change.** Only fall back to a
constructor if it provably cannot work — see STOP triggers.

## Change

### 1. `crates/server/Cargo.toml` — dev-dependencies

Add to the existing `[dev-dependencies]` block (which already has `db` and `serial_test`):

```toml
wiremock = "0.6"
tempfile = "3"
jsonwebtoken = { version = "10.2.0", features = ["rust_crypto"] }
```

`jsonwebtoken` is required by Amendment B.3 — the mocked access token must be a decodable JWT.
The version matches `crates/utils/Cargo.toml`'s existing entry, so no new version enters the lock.

Do **not** add `tower` — see "Amendment A" below; the harness binds a real listener instead of
using `ServiceExt::oneshot`. Do not add anything to `[dependencies]` — this is test-only.
`reqwest` is already a direct dependency of `crates/server` (`Cargo.toml:47`), so it is usable
from tests without any manifest change.

`Cargo.lock` will change as a byproduct of adding the two dev-dependencies. That is expected and
`Cargo.lock` is listed in `files:`. Do not hand-edit it — let cargo write it.

## Amendment A (orchestrator, 2026-07-30) — drive a REAL bound listener, not `oneshot`

**This supersedes any instruction below to use `tower::ServiceExt::oneshot`.**

The original text said to "build the router the same way `crates/server/src/routes/mod.rs` does"
and drive it with `oneshot`. That is **impossible**: the only public constructor is

```rust
pub async fn router(deployment: DeploymentImpl) -> IntoMakeService<Router>   // routes/mod.rs:38
```

which ends in `.into_make_service()` (`routes/mod.rs:78`). `IntoMakeService` is a *MakeService*,
not a `Service<Request>`, so `oneshot` does not apply to it, and it exposes **no** `into_inner()`
(verified: `error[E0599]: no method named 'into_inner' found for struct 'IntoMakeService<S>'`).

The two ways out are not equal:

- **Hand-assembling a `Router` inside the harness is FORBIDDEN.** A harness that merges the
  route modules itself would make tasks 101-104's registration tests pass *by construction* —
  the test would merge the very router it claims to be testing, so a missing
  `.merge(nodes::router())` in `routes/mod.rs` would be invisible. That missing registration is
  the exact bug this workstream exists to fix.
- **Adding a production accessor** that returns the inner `Router` is also forbidden here — this
  task must not change production code.

So: **bind a real ephemeral TCP listener and serve the real `server::routes::router()`, exactly as
`crates/server/src/main.rs:207,273` does, then make real HTTP requests with `reqwest`.** This is
strictly stronger than `oneshot` for this workstream — it exercises the true production entry
point, route registration included.

```rust
let app = server::routes::router(deployment).await;          // the REAL constructor, unmodified
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap();
tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
// requests: reqwest::Client::new().get(format!("http://{addr}{path}")).send().await
```

Store `addr` in `HiveHarness`; `get`/`post` build their URL from it. The dictated public surface
(`configured`, `hive_absent`, `mock_json`, `get`, `post`, `Resp`) is **unchanged** — only the
transport beneath it changes, so tasks 101-104 and 301 are unaffected.

### Amendment A.1 — Rust 2024 requires `unsafe` for env mutation

`std::env::set_var` / `remove_var` are `unsafe` in edition 2024 (this crate is edition 2024):
`error[E0133]: call to unsafe function 'set_var' is unsafe and requires unsafe block`. Wrap each
call in an `unsafe { … }` block. This is why every consuming test must be `#[serial_test::serial]`.

### Amendment A.2 — wiremock matcher shape

Compose matchers with `.and(...)`, never `.expect(...)` (`expect` takes a call-count, which
produced `error[E0277]: the trait bound 'Times: From<PathExactMatcher>' is not satisfied`):

```rust
wiremock::Mock::given(wiremock::matchers::method(method))
    .and(wiremock::matchers::path(path))
    .respond_with(wiremock::ResponseTemplate::new(status).set_body_json(body))
    .mount(&mock_server)
    .await;
```

### 2. Create `crates/server/tests/common/mod.rs`

A harness exposing exactly this surface (implementation is yours; the surface is dictated because
tasks 101-104 and 301 are written against it):

```rust
pub struct HiveHarness { /* mock server, temp dir, deployment */ }

pub struct Resp { pub status: u16, pub body: String }

impl HiveHarness {
    /// VK_SHARED_API_BASE points at a live wiremock server -> hive IS configured.
    pub async fn configured() -> Self;
    /// VK_SHARED_API_BASE unset -> deployment.remote_client() is Err(RemoteClientNotConfigured).
    pub async fn hive_absent() -> Self;
    /// Queue a canned hive response. `path` is the hive-side path (e.g. "/v1/nodes").
    pub async fn mock_json(&self, method: &str, path: &str, status: u16, body: serde_json::Value);
    /// Drive the REAL served router over HTTP (see Amendment A), not a handler call.
    pub async fn get(&self, path: &str) -> Resp;
    pub async fn post(&self, path: &str, body: serde_json::Value) -> Resp;
}
```

### Construction sequence — THE authoritative ordering

Both constructors follow this list. It supersedes any ordering implied elsewhere in this file;
where an Amendment says "before building the deployment", this list is where it goes.

1. `let temp_dir = tempfile::TempDir::new().unwrap();`
2. Env hygiene per **Amendment A.4** — remove `VK_HIVE_URL` and `VK_NODE_API_KEY`, set the two
   `DISABLE_WORKTREE_*` vars.
3. Redirect all on-disk state into the temp dir:
   ```rust
   unsafe { std::env::set_var("VK_ASSET_DIR", temp_dir.path()) };
   unsafe { std::env::set_var("VK_DATABASE_PATH", temp_dir.path().join("db.sqlite")) };
   ```
   `VK_ASSET_DIR` (Amendment B.1) moves `config.json` and `credentials.json`; `VK_DATABASE_PATH`
   wins over it for the database (`crates/utils/src/assets.rs:48-59`), so both are set.
4. **`configured()` only** — seed `credentials.json` into `temp_dir` (**Amendment B.2**). This must
   happen before step 7, because credentials are loaded during `Deployment::new()`.
5. **`configured()` only** — `let mock_server = wiremock::MockServer::start().await;` and mount the
   `/v1/tokens/refresh` mock (**Amendment B.3**).
6. `configured()`: `unsafe { std::env::set_var("VK_SHARED_API_BASE", mock_server.uri()) };`
   `hive_absent()`: `unsafe { std::env::remove_var("VK_SHARED_API_BASE") };`
7. Build the deployment the same way `crates/server/src/main.rs` does:
   `local_deployment::LocalDeployment::new().await`. **`new()` is a TRAIT method** on
   `deployment::Deployment` (`crates/deployment/src/lib.rs:77`, implemented at
   `crates/local-deployment/src/lib.rs:101`), so the harness MUST have `use deployment::Deployment;`
   in scope or the call will not resolve. The doc example at
   `crates/local-deployment/src/lib.rs:95-100` shows exactly this usage.
8. **`hive_absent()` only** — the **Amendment A.3** assertion.
9. Serve the real `server::routes::router(deployment).await` on an ephemeral listener per
   **Amendment A**. Store the bound `SocketAddr`, the `TempDir`, and the `MockServer` in the
   struct — dropping the `TempDir` early would delete the seeded credentials out from under the
   running server.

`hive_absent()` also starts a `MockServer` (see Amendment A.6); it simply never points
`VK_SHARED_API_BASE` at it, and it skips step 4.

### Amendment A.3 — `hive_absent()` must ASSERT absence, never assume it

Removing the env var is **not sufficient** to guarantee the hive is unconfigured. The resolution
at `crates/local-deployment/src/lib.rs:188-190` is:

```rust
let api_base = std::env::var("VK_SHARED_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_API_BASE").map(|s| s.to_string()));
```

`option_env!` is a **compile-time** lookup, baked in by `crates/server/build.rs:10-12`, which
itself calls `dotenv::dotenv()`. So if `VK_SHARED_API_BASE` is ever set in the environment or
uncommented in `.env` (currently commented at `.env:95-96`) **when the test binary is built**,
`hive_absent()` would silently produce a CONFIGURED deployment — and every "absent hive" test in
tasks 101-104 and 402 would quietly assert the wrong thing while still passing.

`hive_absent()` MUST therefore assert the precondition it claims, immediately after building the
deployment and before serving:

```rust
assert!(
    deployment.remote_client().is_err(),
    "hive_absent() built a CONFIGURED deployment — VK_SHARED_API_BASE was baked in at compile \
     time via build.rs/option_env!. Unset it (and check .env) and rebuild."
);
```

This turns a silent false-green into a loud, self-explaining failure. Keep the message.

Every test using the harness MUST be `#[serial_test::serial]` — the harness mutates process-wide
environment variables, and parallel tests would race.

### Amendment A.6 — two shapes the task must not leave you to guess

- **`hive_absent()` DOES start a `MockServer`.** The field is therefore non-`Option` and
  `mock_json` compiles identically on both constructors; `hive_absent()` simply never points
  `VK_SHARED_API_BASE` at it. Calling `mock_json` on an absent-hive harness is legal and has no
  effect. (Do not model the mock server as `Option`.)
- **Put `#[allow(dead_code)]` on the `impl HiveHarness` block and on `struct Resp`.**
  `crates/server/tests/common/mod.rs` is compiled separately into EACH integration-test binary, so
  any method that binary does not call is dead code there. `harness_smoke.rs` never calls `post`,
  so without the attribute `cargo clippy … -D warnings` fails on
  `method 'post' is never used` and the Manual verification below cannot pass.

**Requests must traverse the mounted router.** The harness exists precisely so tests do not call
handler functions directly — a direct call cannot prove registration, which is the bug this whole
workstream fixes. For the same reason the harness must obtain its router **only** from
`server::routes::router(...)`; see Amendment A.

### Amendment A.4 — required env hygiene BEFORE building the deployment

Both `configured()` and `hive_absent()` MUST apply this before calling `Deployment::new()`. These
are env vars, not production-code changes, so they are explicitly within Allowed moves:

```rust
unsafe { std::env::remove_var("VK_HIVE_URL") };      // else Deployment::new() spawns a persistent
unsafe { std::env::remove_var("VK_NODE_API_KEY") };  // node-runner websocket loop that never ends
                                                     // (local-deployment/src/lib.rs:257-260 ->
                                                     //  NodeRunnerConfig::from_env, node_runner.rs:61-63)
unsafe { std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1") };   // container.rs:322
unsafe { std::env::set_var("DISABLE_WORKTREE_EXPIRED_CLEANUP", "1") };  // container.rs:447
```

`configured()` sets `VK_SHARED_API_BASE`; if the developer's environment ALSO has `VK_HIVE_URL`,
the node runner starts. Removing both vars is not optional — this is the most likely cause of the
"test hangs" STOP trigger below.

Do **not** set `VK_SCHEDULED_BACKUPS` — it is inert here. `BackupScheduler::spawn` has no call
site outside `crates/db/src/backup_scheduler.rs` (the only other mention is the re-export at
`crates/db/src/lib.rs:28`), so `Deployment::new()` never starts it. The same holds for the WAL
monitor, so CLAUDE.md's `VK_WAL_*` variables are irrelevant to this harness.

### Amendment A.5 — why `VK_ASSET_DIR` is mandatory (the rationale behind Amendment B.1)

`VK_DATABASE_PATH` isolates only the database. `Deployment::new()` **also unconditionally
rewrites** `asset_dir()/config.json` — `save_config_to_file(&raw_config, &config_path).await?`
(`crates/local-deployment/src/lib.rs:133`, commented "Always save config"), and it may mutate
`show_release_notes` / `last_app_version` (`:119-129`). `asset_dir()` has **no** env override:
under `debug_assertions` it is hard-wired to `dev_assets/` (`crates/utils/src/assets.rs:6-14`).

This is exactly why Amendment B.1 sets `VK_ASSET_DIR` to the harness's `TempDir`: with it set,
`config.json` is written inside the temp directory and `dev_assets/` is left alone. Setting
`VK_ASSET_DIR` is therefore mandatory, not optional. The Manual verification below asserts
`dev_assets/config.json` is untouched by a test run.

## Amendment B (orchestrator, 2026-07-30) — the OAuth seam, via task 099's `VK_ASSET_DIR`

**Without this, `configured()` returns 401 and the frozen spec's `200` assertion is unreachable.**

Pointing `VK_SHARED_API_BASE` at wiremock is NOT sufficient. `/api/organizations` — and every
proxy restored in tasks 101-104, and the handler in 301 — is OAuth-authed:

- `crates/server/src/routes/organizations.rs:69-76` → `deployment.remote_client()?` →
  `client.list_organizations()`
- `crates/services/src/services/remote_client.rs:541-543` → `self.get_authed("/v1/organizations")`
- `crates/services/src/services/remote_client.rs:242-246` →
  `auth_context.get_credentials().await.ok_or(RemoteClientError::Auth)?`

With no credentials the request never reaches wiremock: `RemoteClientError::Auth` maps to
`StatusCode::UNAUTHORIZED` (`crates/server/src/error.rs:168`). Task **099** makes
`credentials_path()` = `asset_dir()/credentials.json` redirectable, which is why this task now
depends on it.

### B.1 — point the asset root at the TempDir

Before building the deployment (alongside the Amendment A.4 block):

```rust
unsafe { std::env::set_var("VK_ASSET_DIR", temp_dir.path()) };
```

This also removes the side effect Amendment A.5 describes: `config.json` is written inside the
`TempDir`, not into the repo's `dev_assets/`. Read A.5 — it is the rationale for this step, not a
dead section.

### B.2 — seed the credentials file

Write `credentials.json` into that directory **before** `Deployment::new()` (the credentials are
loaded during construction: `credentials_path()` is resolved at
`crates/local-deployment/src/lib.rs:104` and `creds.load().await` runs at `:108`).

The on-disk format is `StoredCredentials`, which holds **only** a refresh token
(`crates/services/src/services/oauth_credentials.rs:25-28`):

```rust
std::fs::write(
    temp_dir.path().join("credentials.json"),
    r#"{"refresh_token":"test-refresh-token"}"#,
).unwrap();
```

On Linux the store is always the file backend; on macOS it is the file backend under
`debug_assertions` (`oauth_credentials.rs:92-105`), so tests use the file on both.

### B.3 — mock the token refresh endpoint (REQUIRED, and easy to miss)

`StoredCredentials` carries no access token, so `Credentials::from` yields
`access_token: None, expires_at: None` (`oauth_credentials.rs:30-38`) and
`expires_soon()` returns `true` via its `_ => true` arm (`:16-23`). **Every** request therefore
takes the refresh path: `refresh_credentials` → `refresh_token_request` →
`post_public("/v1/tokens/refresh")` (`remote_client.rs:330-340`).

That request goes to `VK_SHARED_API_BASE` — i.e. to wiremock — so `configured()` MUST register
this mock itself, before any test-supplied mock:

**The `access_token` in that mock MUST be a real, well-formed JWT with a future `exp` claim — a
plain string like `"test-access-token"` yields a `502`, not a `200`.** After the refresh response
comes back, `refresh_credentials` calls `extract_expiration(&access_token)`
(`crates/services/src/services/remote_client.rs:312`), which is
`jsonwebtoken::dangerous::insecure_decode::<ExpClaim>(token)` (`crates/utils/src/jwt.rs:22-26`).
A non-JWT fails to decode → `TokenClaimsError::Decode` → `RemoteClientError::Token(...)` →
`StatusCode::BAD_GATEWAY` (`crates/server/src/error.rs:175`).

The signature is never verified, so any secret works — but the three-part JWT structure and the
`exp` claim are mandatory. Copy the repo's own helper, `make_jwt_with_exp`
(`crates/utils/src/jwt.rs:34-48`); do not invent one:

```rust
/// The access token MUST be a real JWT with a future `exp`: RemoteClient calls
/// utils::jwt::extract_expiration() on it (remote_client.rs:312), which uses
/// jsonwebtoken::dangerous::insecure_decode. A plain string yields
/// RemoteClientError::Token -> HTTP 502, not 200. The signature is NOT verified,
/// so any secret works, but the JWT structure and the `exp` claim are mandatory.
/// Mirrors crates/utils/src/jwt.rs:34-48.
fn test_access_token() -> String {
    #[derive(serde::Serialize)]
    struct Claims {
        exp: usize,
    }

    let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims { exp },
        &jsonwebtoken::EncodingKey::from_secret(b"test-secret"),
    )
    .expect("failed to encode test JWT")
}
```

```rust
wiremock::Mock::given(wiremock::matchers::method("POST"))
    .and(wiremock::matchers::path("/v1/tokens/refresh"))
    .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "access_token": test_access_token(),
        "refresh_token": "test-refresh-token"
    })))
    .mount(&mock_server)
    .await;
```

The response shape is `TokenRefreshResponse { access_token, refresh_token }`
(`crates/utils/src/api/oauth.rs:41-46`) — both fields are required and non-optional.

Do NOT bound this mock with `.up_to_n_times(...)`: the refresh fires on every authed call, and the
number of calls is not something a test should have to predict.

This needs ONE new entry in `crates/server/Cargo.toml` `[dev-dependencies]` (already in `files:`):

```toml
jsonwebtoken = { version = "10.2.0", features = ["rust_crypto"] }
```

`chrono` is already a regular dependency of `crates/server` (`Cargo.toml:38`), and `serde` is too,
so neither needs adding.

### B.4 — `hive_absent()` does NOT seed credentials

It sets `VK_ASSET_DIR` to its own `TempDir` (for config isolation) but writes no
`credentials.json`. Its assertions are about the not-configured path, which is reached before any
auth check, so seeding would only obscure what is being tested.

## Allowed moves

- Only the four files in `files:`. If `Deployment::new()` needs another env var to run in a test
  (config path, data dir), set it in the harness — do not change production code to accommodate
  the test.

## STOP triggers

- **If `Deployment::new()` cannot be driven from a test** — it spawns something that will not
  terminate, requires a real network, or panics without a full runtime — STOP and report exactly
  what blocked it. Do NOT start refactoring `LocalDeployment`. The fallback (a `test-utils`
  feature on `crates/local-deployment` exposing a minimal constructor) is a **separate task the
  orchestrator will author**, because it changes production types and needs its own review.
- If the harness would need any change under `crates/local-deployment/src/` or
  `crates/services/src/` — STOP. That is the fallback path, not this task.
- If a test hangs, STOP and report rather than adding a timeout that masks it.
- **If `harness_smoke` returns `502` rather than `200`, the mocked `access_token` is not a
  decodable JWT** (Amendment B.3). Fix the token — do NOT relax the `200` assertion.
- If it returns `401`, credential seeding did not take effect: check that `VK_ASSET_DIR` is set
  and `credentials.json` written BEFORE `Deployment::new()`. Again, do not relax the assertion.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cargo test -p server --test harness_smoke
# Expected: 2 passed

cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

git diff --stat crates/local-deployment crates/services crates/utils
# Expected: NO output (production crates untouched). Task 099 owns crates/utils; this check
#           assumes 099 is COMMITTED, i.e. in HEAD. If 099 is applied but uncommitted, commit
#           it before running this — otherwise its diff shows up here and masks the result.

git status --porcelain dev_assets/
# Expected: NO output — VK_ASSET_DIR (Amendment B.1) kept the test run out of dev_assets/.
#           A modified dev_assets/config.json here means VK_ASSET_DIR was not set before
#           Deployment::new(), and is a FAILURE of this task.
```

## Done when

- `HiveHarness::configured()` and `::hive_absent()` both build a real `DeploymentImpl` and expose
  the mounted router.
- `harness_smoke.rs` passes both tests against `/api/organizations`, including the **`200` +
  `success: true`** assertion — the frozen spec's SC1/SC4 signal, reachable only because
  Amendment B seeds credentials and mocks `/v1/tokens/refresh`.
- A test run leaves `dev_assets/` untouched.
- No file outside `crates/server/tests/`, `crates/server/Cargo.toml`'s `[dev-dependencies]`, and
  the cargo-written `Cargo.lock` changed.
- The anti-fake check (Amendment A) passes:

  ```bash
  grep -nE 'routes::router\s*\(' crates/server/tests/common/mod.rs
  # Expected: a hit

  grep -nE '\.merge\(|\.route\(|\.nest\(|Router::new\(' crates/server/tests/common/mod.rs
  # Expected: NO output — the harness must not assemble a router of its own

  grep -nE 'routes::(config|containers|dashboard|projects|tasks|organizations|nodes)::' \
    crates/server/tests/common/mod.rs
  # Expected: NO output — no per-module router may be referenced
  ```
