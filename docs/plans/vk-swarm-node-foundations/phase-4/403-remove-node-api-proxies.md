---
id: "403"
phase: 4
title: Remove pure-proxy remote API modules (nodes, swarm_*, merged-projects)
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/server/src/routes/mod.rs
  - crates/server/src/routes/nodes.rs
  - crates/server/src/routes/swarm_projects.rs
  - crates/server/src/routes/swarm_labels.rs
  - crates/server/src/routes/swarm_templates.rs
  - crates/server/src/routes/projects/mod.rs
  - crates/server/src/routes/projects/handlers/mod.rs
  - crates/server/src/routes/projects/handlers/merged.rs
irreversible: true
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---
## Failing test (write first)
`N/A — covered by: the route-table compile + `cargo test -p server --no-run``. This deletes public
routes; correctness = "the pure-proxy routes are gone and the server still builds (incl. its test
modules)". Verified by the `## Manual verification` section + the gate.

> **🚧 HUMAN GATE (irreversible):** removes public API routes (`/api/nodes*`, `/api/swarm/*`,
> `/api/merged-projects`) — an API contract change. A `reviews/403.approved` token must exist first.

> **SCOPE — narrowed after breakdown review (R4/R5).** The two task remote-stream routes
> (`/available-nodes`, `/stream-connection-info`) are **NOT removed here**: review found they still
> have **live callers** — the MCP `list_nodes` tool (`crates/server/src/mcp/task_server.rs:1390`) and
> the local frontend hooks `useAvailableNodes` (`CreateAttemptDialog`) / `useRemoteConnectionStatus`
> (`AttemptHeaderActions`). Removing them strands those callers. They are deferred to the
> `vk-swarm-node-ui-localize` workstream, together with their frontend consumers. This task removes ONLY
> the self-contained pure-proxy modules whose routes have no surviving local caller.

> **Codegen boundary (403↔404):** this task does NOT run `npm run generate-types` and does NOT remove
> any `#[ts(export)]` struct (`MergedProject`/`MergedProjectsResponse` stay in `projects/types.rs`).
> They become unreferenced backend types but keep compiling + exporting to TS, so the frontend stays
> green independently. The Rust gate skips the TS type-check (Trap 1), so keeping the structs is what
> makes 403 safe in isolation. Their frontend consumer is removed later (ui-localize).

## Change

**A. Delete whole proxy modules (`git rm`):**
```text
git rm crates/server/src/routes/nodes.rs
git rm crates/server/src/routes/swarm_projects.rs
git rm crates/server/src/routes/swarm_labels.rs
git rm crates/server/src/routes/swarm_templates.rs
git rm crates/server/src/routes/projects/handlers/merged.rs
```

**B. `crates/server/src/routes/mod.rs` — drop the module decls and router merges.**
- Remove `pub mod nodes;` (L27) and the `pub mod swarm_labels; / swarm_projects; / swarm_templates;`
  block (L32–34).
- Remove these `.merge(...)` lines from `base_routes` (L65–68):
```text
        .merge(nodes::router())
        .merge(swarm_projects::router())
        .merge(swarm_labels::router())
        .merge(swarm_templates::router())
```

**C. `crates/server/src/routes/projects/mod.rs` — drop the merged-projects route + import.**
- Remove `get_merged_projects,` and its `// Merged handlers` comment from the `use handlers::{...}`
  block (L36–37).
- Remove the route (L148): `.route("/merged-projects", get(get_merged_projects))`. Ensure the builder
  chain ends cleanly (no dangling `.`; the chain should terminate on `.nest("/projects", projects_router)`).

**D. `crates/server/src/routes/projects/handlers/mod.rs` — drop the merged module + re-export.**
- Remove `pub mod merged;` (L14) and `pub use merged::get_merged_projects;` (L29).

## Allowed moves
- Delete the five pure-proxy files via `git rm`; edits confined to the three remaining listed files,
  ONLY to remove the named module decls / merges / route / re-exports.
- KEEP every non-proxy route and ALL of `tasks/*` (the two remote-stream routes,
  `resolve_remote_project_id` and its 4 colocated tests at `streams.rs:164–219`, `stream_tasks_ws`).
  Do NOT touch `tasks/mod.rs`, `tasks/handlers/mod.rs`, or `tasks/handlers/streams.rs`.
- Do NOT remove `MergedProject`/`MergedProjectsResponse` from `projects/types.rs` (not in `files:`); do
  NOT run generate-types.
- Do NOT touch the sync layer (SC5d): no `upsert_remote_task`, publisher, WS node runner, or
  `remote_*`/`shared_task_id` writer.

## STOP triggers
- A deleted module's `router()`/import turns out to be referenced by a KEPT route after deletion — STOP;
  remove only what is truly orphaned within the listed files.
- Removing a `.route(...)`/`.merge(...)` leaves a syntactically broken builder chain.
- `cargo test -p server --no-run` reports a dead-code/unused-import error OR a broken test module from a
  deletion — fix within the listed files only; if the fix needs an unlisted file, STOP (it likely
  belongs to ui-localize).
- A deleted module turns out to back a still-needed LOCAL feature — re-verify the inventory before
  deleting (nodes/swarm_*/merged-projects were confirmed pure Hive proxies with no local caller; the
  two task stream routes that DID have local callers are explicitly kept above).

## Manual verification (record in decisions-ledger)
1. `git grep -nE '/api/(nodes|swarm)|merged-projects' crates/server/src/routes`
   → NO `.route(`/`.merge(...router())` registrations remain (matches only in `types.rs`/docs are fine).
2. `cargo test -p server --no-run` → exits 0 (compiles incl. test modules — catches any colocated-test
   breakage a bare `cargo check` would miss).
3. `git grep -nF 'pub mod nodes;' crates/server/src/routes/mod.rs` → no output.
   Record in the ledger the exact route list removed AND that `/available-nodes` +
   `/stream-connection-info` were intentionally KEPT (deferred to `vk-swarm-node-ui-localize`).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p server" WAI_TEST_CMD="cargo test -p server --no-run" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 403` exits 0

> Trap 1: server crate. `WAI_TEST_CMD` is `cargo test --no-run` (NOT bare `cargo check`) so the gate
> COMPILES `#[cfg(test)]` modules — a bare `cargo check` would pass green while a deleted symbol broke a
> colocated test (breakdown-review R5). No new SQL → no `.sqlx` regen.
