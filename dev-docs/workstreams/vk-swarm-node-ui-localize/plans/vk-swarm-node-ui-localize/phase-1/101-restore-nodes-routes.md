---
id: "101"
phase: 1
title: "Restore crates/server/src/routes/nodes.rs (without the API-key routes) and register it"
status: passed
depends_on: ["100"]
parallel: false
conflicts_with: ["102","103","104"]
files:
  - crates/server/src/routes/nodes.rs
  - crates/server/tests/nodes_routes.rs
  - crates/server/src/routes/mod.rs
siblings:
  - crates/server/src/routes/organizations.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC1, SC3]
---

## Failing test (write first)

Create `crates/server/tests/nodes_routes.rs`, using the harness from task 100. This is the frozen
spec's required per-module proxy test — hive-configured returns 200 + `success: true`, hive-absent
returns the not-configured variant rather than a 500, and **both drive the mounted router**:

```rust
mod common;

#[tokio::test]
#[serial_test::serial]
async fn configured_hive_returns_success() {
    let h = common::HiveHarness::configured().await;
    h.mock_json("GET", "/v1/nodes", 200, serde_json::json!([])).await;
    let res = h.get("/api/nodes?organization_id=00000000-0000-0000-0000-000000000000").await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);
    assert!(res.body.contains("\"success\":true"), "body: {}", res.body);
}

#[tokio::test]
#[serial_test::serial]
async fn absent_hive_is_registered_and_not_a_500() {
    let h = common::HiveHarness::hive_absent().await;
    let res = h.get("/api/nodes?organization_id=00000000-0000-0000-0000-000000000000").await;
    res.assert_registered();
    assert_ne!(res.status, 500, "absent hive is a client-visible state, not a server error");
}
```

Adapt the mocked hive path and JSON body to what this module's `RemoteClient` method actually
requests and deserialises — read the method in
`crates/services/src/services/remote_client.rs` first. If the mocked body does not deserialise,
the configured-hive test fails loudly; do NOT weaken it to only assert a status code.


> [!WARNING]
> **Registration is NOT proved by a non-404 status in this codebase.** The outer router ends in a
> catch-all `.route("/{*path}", get(frontend::serve_frontend))`
> (`crates/server/src/routes/mod.rs:76`), and `serve_frontend` returns `StatusCode::OK` with
> `index.html` for unknown routes (`crates/server/src/routes/frontend.rs:40-43`). An UNREGISTERED
> `/api/...` GET therefore returns **200 + SPA HTML**, never 404 — verified empirically. Use
> `Resp::assert_registered()` (task 100, Amendment C.1), which fails when the response is the SPA
> fallback. Never assert `assert_ne!(status, 404)` to mean "registered".

## Sibling alignment (required reading before you write)

Read `crates/server/src/routes/organizations.rs`. It is the live, shipped example of this exact
pattern (a `RemoteClient` proxy router in this crate). List every structural choice it makes —
how it obtains the client (`deployment.remote_client()?`), how it wraps
(`ResponseJson(ApiResponse::success(..))`), how the router is built and exported. The restored
module must match those choices. Record any divergence in the decisions-ledger.

## Change

### 1. Create `crates/server/src/routes/nodes.rs`

Recover the file verbatim, then delete the API-key parts:

```bash
git show 35b378a5^:crates/server/src/routes/nodes.rs > crates/server/src/routes/nodes.rs
```

Then make exactly these four deletions (decision D3 / ADR-0013 — the hive owns API keys):

**(a)** Delete the `ListApiKeysQuery`, `CreateApiKeyRequest`, and `CreateApiKeyResponse` structs:

```rust
#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub organization_id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: NodeApiKey,
    pub secret: String,
}
```

**(b)** Delete the three handler functions `list_api_keys`, `create_api_key`, and
`revoke_api_key` (each begins with its `/// ...` doc comment and ends at its closing `}`).

**(c)** In `pub fn router()`, delete these two lines:

```rust
        .route("/nodes/api-keys", get(list_api_keys).post(create_api_key))
        .route("/nodes/api-keys/{key_id}", delete(revoke_api_key))
```

so the router body is exactly:

```rust
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/{node_id}", get(get_node).delete(delete_node))
        .route("/nodes/{node_id}/projects", get(list_node_projects))
}
```

**(d)** Fix the now-unused imports. After (a)–(c), `Json`, `Serialize`, and `NodeApiKey` are
unused. The import block must end up exactly:

```rust
use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use remote::nodes::{Node, NodeLocalProjectInfo};
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
```

(`delete` leaves the `routing::` import because `delete_node` is now reached via
`get(get_node).delete(delete_node)`, which is a method-router call, not the `routing::delete`
function.)

### 2. Register it in `crates/server/src/routes/mod.rs`

- **Anchor:** the `pub mod` declaration block (alphabetical, around line 30)
- **Before:** `pub mod projects;`
- **After:**
```rust
pub mod nodes;
pub mod projects;
```

- **Anchor:** the `base_routes` builder, line 60
- **Before:** `        .merge(organizations::router())`
- **After:**
```rust
        .merge(organizations::router())
        .merge(nodes::router())
```

## Allowed moves

- Create `crates/server/src/routes/nodes.rs` as described.
- Add the `pub mod nodes;` declaration and the single `.merge(nodes::router())` line.

## STOP triggers

- If `crates/server/src/routes/nodes.rs` already exists on disk.
- If `git show 35b378a5^:crates/server/src/routes/nodes.rs` fails or returns a file whose
  `router()` does not match the five routes listed above.
- If any import in the recovered file fails to resolve (e.g. `remote::nodes::Node` is gone) —
  do NOT invent a replacement type; STOP.
- If `cargo check` reports an error you would have to fix by changing a file not in `files:`.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cargo check -p server
# Expected: finishes with no errors and no `unused import` warnings for routes/nodes.rs

cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

grep -n 'api-keys' crates/server/src/routes/nodes.rs
# Expected: NO output (D3 — the API-key routes must not come back)
```

## Done when

- `crates/server/src/routes/nodes.rs` exists with exactly three routes: `/nodes`,
  `/nodes/{node_id}`, `/nodes/{node_id}/projects`.
- No API-key route, handler, or request/response struct survives in that file.
- `routes/mod.rs` declares `pub mod nodes;` and merges `nodes::router()` into `base_routes`.
- `cargo check -p server` and `cargo clippy -p server ... -D warnings` are clean.
