---
id: "104"
phase: 1
title: "Restore crates/server/src/routes/swarm_templates.rs verbatim and register it"
status: passed
depends_on: ["103","100"]
parallel: false
conflicts_with: ["101","102","103"]
files:
  - crates/server/src/routes/swarm_templates.rs
  - crates/server/tests/swarm_templates_routes.rs
  - crates/server/src/routes/mod.rs
siblings:
  - crates/server/src/routes/organizations.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC1]
---

## Failing test (write first)

Create `crates/server/tests/swarm_templates_routes.rs`, using the harness from task 100. This is the frozen
spec's required per-module proxy test — hive-configured returns 200 + `success: true`, hive-absent
returns the not-configured variant rather than a 500, and **both drive the mounted router**:

```rust
mod common;

#[tokio::test]
#[serial_test::serial]
async fn configured_hive_returns_success() {
    let h = common::HiveHarness::configured().await;
    h.mock_json("GET", "/v1/swarm/templates", 200, serde_json::json!({"templates": []})).await;
    let res = h.get("/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000").await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);
    assert!(res.body.contains("\"success\":true"), "body: {}", res.body);
}

#[tokio::test]
#[serial_test::serial]
async fn absent_hive_is_registered_and_not_a_500() {
    let h = common::HiveHarness::hive_absent().await;
    let res = h.get("/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000").await;
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

## Amendments (ORCHESTRATOR, pre-dispatch — these are DICTATED, not choices)

**D1 — mocked body shape.** The original draft mocked `serde_json::json!([])`. That does NOT
deserialise: `list_swarm_templates` returns `ListSwarmTemplatesResponse`, a struct with a
`templates` field (`crates/remote/src/routes/swarm_templates.rs:82-84`), not a bare array. The test
block above has been corrected to `json!({"templates": []})`. Use it as written. Tasks 102 and 103
hit the identical defect; both panels proved by revert experiment that the wrong body yields
`400 "Unexpected response from remote service."`, so this correction is load-bearing.

**D2 — mocked hive path is correct as written.** `RemoteClient::list_swarm_templates`
(`crates/services/src/services/remote_client.rs:1327-1335`) requests
`/v1/swarm/templates?organization_id=...` unconditionally, with no `AuthMode` branch.
`wiremock::matchers::path` ignores the query string (`crates/server/tests/common/mod.rs:170-176`),
so mocking `"/v1/swarm/templates"` matches. Do NOT add the query string to the mock.

**D3 — TWO DIFFERENT ORDERINGS, do not "fix" either.** This is the deviation task 103's
implementer made and had to correct:
- the `pub mod` block is **ALPHABETICAL** → `pub mod swarm_templates;` goes AFTER
  `pub mod swarm_projects;` (labels, projects, templates — alphabetical).
- the `.merge(...)` chain is **TASK-ORDERED**, NOT alphabetical → append
  `.merge(swarm_templates::router())` immediately AFTER `.merge(swarm_labels::router())`, so the
  chain reads `nodes → swarm_projects → swarm_labels → swarm_templates`.

Do not alphabetise the merge chain. Do not reorder anything else in either block.

**D4 — run `cargo fmt --all` before reporting**, then confirm the recovered module is STILL
byte-identical. Report the REAL exit code of `cargo fmt --all -- --check` captured as
`cargo fmt --all -- --check > /tmp/f.txt 2>&1; echo $?` — NOT `$?` after a pipe to `tail`, which
reports tail's status. The `Warning: can't set group_imports/imports_granularity` lines are
pre-existing nightly noise; only `Diff in` lines are failures.

**D5 — the LEDGER is not "empty by default".** Before declaring it empty, diff your own change
against the task text line by line. Task 103's implementer declared `empty` while having
alphabetised the merge chain. An empty ledger is the win condition ONLY when you actually followed
the text; an empty ledger that hides a deviation is the failure this whole process exists to catch.

## Sibling alignment (required reading before you write)

Read `crates/server/src/routes/organizations.rs` — the live example of this pattern in this
crate. The restored module must obtain its client via `deployment.remote_client()?` and wrap
responses in `ResponseJson(ApiResponse::success(..))` exactly as it does. Record any divergence
in the decisions-ledger.

## Change

### 1. Create `crates/server/src/routes/swarm_templates.rs` — VERBATIM, no edits

```bash
git show 35b378a5^:crates/server/src/routes/swarm_templates.rs > crates/server/src/routes/swarm_templates.rs
```

Do **not** reformat, rename, or "improve" anything in the recovered file. Every type and
`RemoteClient` method it imports was verified present on `main` during decomposition. Its
`router()` registers:

- `/swarm/templates` (list, create)
- `/swarm/templates/{template_id}` (get, update, delete)
- `/swarm/templates/{template_id}/merge`

### 2. Register it in `crates/server/src/routes/mod.rs`

- **Anchor:** the `pub mod` declaration block (alphabetical)
- **Change:** add `pub mod swarm_templates;` in alphabetical position.

- **Anchor:** the `base_routes` builder — the `.merge(...)` chain
- **Change:** append one line `.merge(swarm_templates::router())` immediately after the previously added
  `.merge(` line from task 103 (so the four restored modules sit together, in task order).

## Allowed moves

- Create `crates/server/src/routes/swarm_templates.rs` from the git recovery command above.
- Add exactly one `pub mod` line and exactly one `.merge(...)` line to `routes/mod.rs`.

## STOP triggers

- If `crates/server/src/routes/swarm_templates.rs` already exists on disk.
- If the `git show` command fails or produces an empty file.
- If any import in the recovered file fails to resolve — do NOT substitute a different type; STOP.
- If making it compile would require editing a file not listed in `files:`.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cargo check -p server
# Expected: no errors, no unused-import warnings for the new module

cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

git diff --stat crates/server/src/routes/swarm_templates.rs
# Expected: the file is byte-identical to 35b378a5^ (no local edits)
```

## Done when

- `crates/server/src/routes/swarm_templates.rs` exists, byte-identical to `35b378a5^`.
- `routes/mod.rs` declares and merges it.
- `cargo check` and `cargo clippy -D warnings` are clean.
