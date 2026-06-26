---
id: "403"
phase: 4
title: Remove node-surface remote API proxy routes (nodes, swarm, merged-projects, task remote-stream)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/server/src/routes/mod.rs
  - crates/server/src/routes/nodes.rs
  - crates/server/src/routes/swarm_projects.rs
  - crates/server/src/routes/swarm_labels.rs
  - crates/server/src/routes/swarm_templates.rs
  - crates/server/src/routes/tasks/mod.rs
  - crates/server/src/routes/tasks/handlers/mod.rs
  - crates/server/src/routes/tasks/handlers/streams.rs
  - crates/server/src/routes/projects/mod.rs
  - crates/server/src/routes/projects/handlers/mod.rs
  - crates/server/src/routes/projects/handlers/merged.rs
irreversible: true
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---
## Failing test (write first)

`N/A — covered by: crates/db/tests/task_visibility_discriminator.rs` plus the route-table compile.
This is a deletion of public routes; correctness = "the routes are gone and the server still builds".
There is no cheap unit test for "route absent" without standing up the full app. Verified instead by
the `## Manual verification` section below (route-table assertion) + `cargo check -p server` in the gate.

> **🚧 HUMAN GATE (irreversible):** removes public API routes (`/api/nodes*`, `/api/swarm/*`,
> `/api/merged-projects`, `/api/.../available-nodes`, `/api/.../stream-connection-info`) — an API
> contract change. A `reviews/403.approved` token must exist before this runs.

> **Codegen boundary (403↔404):** this task does NOT run `npm run generate-types` and does NOT remove
> any `#[ts(export)]` struct (notably `MergedProject`/`MergedProjectsResponse` stay in
> `projects/types.rs`). They become unreferenced backend types but keep compiling and keep exporting to
> TS, so the frontend stays green independently. The Rust gate skips the TS type-check (Trap 1), so it
> cannot catch frontend breakage — keeping the structs is what makes 403 safe in isolation. Their
> frontend consumer is removed in 404.

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
- Remove these `pub mod` lines (L27, L32–34):
```text
pub mod nodes;
```

```text
pub mod swarm_labels;
pub mod swarm_projects;
pub mod swarm_templates;
```

- Remove these `.merge(...)` lines from `base_routes` (L65–68):
```text
        .merge(nodes::router())
        .merge(swarm_projects::router())
        .merge(swarm_labels::router())
        .merge(swarm_templates::router())
```

**C. `crates/server/src/routes/tasks/mod.rs` — drop the two remote-stream routes.**
- In `task_actions_router`, remove (L51–55):
```text
        .route("/available-nodes", get(handlers::get_available_nodes))
        .route(
            "/stream-connection-info",
            get(handlers::get_stream_connection_info),
        );
```

  and re-terminate the builder: the `.route("/labels", ...)` block above becomes the final call, so
  its closing `)` gains a `;`. Also delete the two matching `///` doc lines (L36–37).

**D. `crates/server/src/routes/tasks/handlers/mod.rs` — drop the re-export (L22).**
- Before: `pub use streams::{get_available_nodes, get_stream_connection_info, stream_tasks_ws};`
- After:  `pub use streams::stream_tasks_ws;`

**E. `crates/server/src/routes/tasks/handlers/streams.rs` — delete the two handlers + now-dead helper + import.**
- Delete `pub async fn get_available_nodes` (L85) and `pub async fn get_stream_connection_info` (L116)
  in full (through their closing braces). KEEP `stream_tasks_ws`.
- Delete the now-orphaned helper `fn resolve_remote_project_id` (L25–38) — used only by the two
  deleted handlers (verify no remaining caller before deleting).
- Delete the now-unused import (L14):
  `use remote::routes::{projects::ListProjectNodesResponse, tasks::TaskStreamConnectionInfoResponse};`
- Remove any other import left unused by the deletions (confirm each against `stream_tasks_ws` before
  removing). The L1 doc comment naming the two handlers may be trimmed.

**F. `crates/server/src/routes/projects/mod.rs` — drop the merged-projects route + import.**
- Remove `get_merged_projects,` and its `// Merged handlers` comment from the `use handlers::{...}`
  block (L36–37).
- Remove the route (L148): `.route("/merged-projects", get(get_merged_projects))`. The router's final
  call becomes `.nest("/projects", projects_router)` — ensure the chain ends cleanly (no dangling `.`).

**G. `crates/server/src/routes/projects/handlers/mod.rs` — drop the merged module + re-export.**
- Remove `pub mod merged;` (L14) and `pub use merged::get_merged_projects;` (L29).

## Allowed moves

- Deletions of the five files via `git rm`; edits confined to the six remaining listed files, ONLY to
  remove the named module decls / merges / routes / re-exports / handlers / now-dead helper + imports.
- KEEP every non-proxy route in the edited files (`stream_tasks_ws`, all `/projects` CRUD/file/github
  routes, all task CRUD/label/archive routes).
- Do NOT remove `MergedProject`/`MergedProjectsResponse`/`NodeLocation`/`TaskCounts` from
  `projects/types.rs` (out of this task's `files:`) and do NOT run generate-types.
- Do NOT touch the sync layer (SC5d): no `upsert_remote_task`, publisher, WS node runner, or
  `remote_*`/`shared_task_id` writer.

## STOP triggers

- A deleted handler's helper/import turns out to also serve a KEPT route (e.g. `resolve_remote_project_id`
  or an import is still referenced after deletion) — STOP; only remove what is truly orphaned.
- Removing a `.route(...)` leaves a syntactically broken builder chain (dangling `.` or missing `;`).
- `cargo check -p server` reports an unused import/dead-code error from a deletion you did not
  anticipate — fix the orphan within the listed files only; if the fix needs an unlisted file, STOP.
- Any swarm/node route turns out to back a still-needed LOCAL feature (it does not — all four modules
  are pure Hive proxies per the inventory) — re-verify before deleting.

## Manual verification (record in decisions-ledger)

1. `git grep -nE '/api/(nodes|swarm)|merged-projects|available-nodes|stream-connection-info' crates/server/src/routes`
   → NO route registrations remain (matches only in `types.rs`/docs are acceptable; no `.route(` or
   `.merge(...router())` hits for these).
2. `cargo check -p server` → exits 0 (no dead-code/unused-import errors).
3. `git grep -nF 'pub mod nodes;' crates/server/src/routes/mod.rs` → no output (module gone).
   Record in the decisions-ledger the exact route list removed (the routes across nodes/swarm_projects/
   swarm_labels/swarm_templates + the 2 task routes + merged-projects).

## Done when

`WAI_TYPECHECK_CMD="cargo check -p server" WAI_TEST_CMD="cargo check -p server" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 403` exits 0

> Trap 1: server crate. No tests added (pure removal), so `WAI_TEST_CMD` reuses `cargo check` to keep
> the gate's "tests pass" step honest without a no-op runner. No new SQL → no `.sqlx` regen.
