---
id: "100"
phase: 1
title: "Build the hive-proxy test harness the frozen spec's Test strategy requires"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/server/tests/common/mod.rs
  - crates/server/tests/harness_smoke.rs
  - crates/server/Cargo.toml
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
- `VK_DATABASE_PATH` overrides the database location (`crates/server/src/main.rs:40`)

**Prefer the env-var route: it requires NO production-code change.** Only fall back to a
constructor if it provably cannot work — see STOP triggers.

## Change

### 1. `crates/server/Cargo.toml` — dev-dependencies

Add to the existing `[dev-dependencies]` block (which already has `db` and `serial_test`):

```toml
tower = { version = "0.5", features = ["util"] }
wiremock = "0.6"
tempfile = "3"
```

`tower`'s `util` feature provides `ServiceExt::oneshot`, which drives a `Router` in-process
without binding a port. Do not add anything to `[dependencies]` — this is test-only.

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
    /// Drive the REAL mounted router via tower's `oneshot`, not a handler call.
    pub async fn get(&self, path: &str) -> Resp;
    pub async fn post(&self, path: &str, body: serde_json::Value) -> Resp;
}
```

Construction sequence for `configured()`:

1. `wiremock::MockServer::start().await`
2. `tempfile::TempDir::new()`, then set `VK_DATABASE_PATH` inside it so the test never touches a
   real database
3. set `VK_SHARED_API_BASE` to the mock server's `uri()`
4. build the deployment via the same path `crates/server/src/main.rs` uses
5. build the router the same way `crates/server/src/routes/mod.rs` does, and keep it for `get`/`post`

`hive_absent()` is identical except `VK_SHARED_API_BASE` is removed.

Every test using the harness MUST be `#[serial_test::serial]` — the harness mutates process-wide
environment variables, and parallel tests would race.

**Requests must traverse the mounted router.** The harness exists precisely so tests do not call
handler functions directly — a direct call cannot prove registration, which is the bug this whole
workstream fixes.

## Allowed moves

- Only the three files in `files:`. If `Deployment::new()` needs another env var to run in a test
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

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cargo test -p server --test harness_smoke
# Expected: 2 passed

cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

git diff --stat crates/local-deployment crates/services
# Expected: NO output (production crates untouched)
```

## Done when

- `HiveHarness::configured()` and `::hive_absent()` both build a real `DeploymentImpl` and expose
  the mounted router.
- `harness_smoke.rs` passes both tests against `/api/organizations`.
- No file outside `crates/server/tests/` and `crates/server/Cargo.toml`'s `[dev-dependencies]`
  changed.
