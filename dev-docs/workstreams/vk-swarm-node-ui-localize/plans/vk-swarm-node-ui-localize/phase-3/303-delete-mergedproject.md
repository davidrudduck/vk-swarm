---
id: "303"
phase: 3
title: "Delete MergedProject, NodeLocation, and the /api/merged-projects endpoint"
status: passed
depends_on: ["302"]
parallel: false
conflicts_with: ["301","302"]
files:
  - crates/server/src/routes/projects/handlers/merged.rs
  - crates/server/src/routes/projects/handlers/mod.rs
  - crates/server/src/routes/projects/types.rs
  - crates/server/src/routes/projects/mod.rs
  - crates/server/src/bin/generate_types.rs
  - crates/server/src/routes/projects/handlers/core.rs
  - crates/server/src/routes/projects/handlers/with_stats.rs
  - frontend/src/lib/api/projects.ts
  - shared/types.ts
irreversible: true
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---

## Failing test (write first)

N/A — deletion. The proof is `cargo clippy -D warnings` + `tsc --noEmit` staying green with the
type gone, the scoped `git grep` over `crates/ frontend/ shared/`, and the SPA-fallback assertion in
Manual verification.

**`forbid_after: ["merged-projects"]` is deliberately NOT set.** It greps every tracked file, and
the term legitimately survives in documentation that must not be rewritten by an implementer:
`dev-docs/adr/0002-node-ui-local-only.md`, `dev-docs/adr/0014-...md`, this workstream's own spec,
the `vk-swarm-node-foundations` plan and ledger, and
`docs/architecture/db/functions/sqlite-project.mdx`. The scoped grep below is the real gate.

## 🚧 IRREVERSIBLE — human gate

Deletes an exported `#[ts(export)]` type and a live HTTP endpoint (ADR-0014). Surface the diff
and the ledger before running.

Note this reverses the *shape* of `a85f7d63`, not its intent: that commit restored the endpoint so
the board would render at all, and task 301 preserved its enrichment verbatim under
`/api/projects/with-stats`. Confirm task 302's board test is green BEFORE running this task — that
test is what proves the board still renders once this endpoint is gone.

## Amendments (ORCHESTRATOR, pre-dispatch — DICTATED)

**G1 — `with_stats.rs` ADDED to `files:`, because its doc comment references what you are deleting.**
Task 301 wrote this header at `crates/server/src/routes/projects/handlers/with_stats.rs:1-6`:

```rust
//! Additive replacement for `merged.rs`'s `MergedProject` shape: the same
//! ... that `MergedProject` still carries (see
//! ADR-0014). `merged.rs` and `/merged-projects` are untouched by this file.
```

Two consequences, both requiring action:
1. The `## Done when` grep (`git grep 'MergedProject\|...\|merged-projects' -- crates frontend shared`
   → expect NO output) would MATCH these comment lines and report failure **even on a perfect
   implementation** — the same impossible-assertion defect corrected at task 302 (F1).
2. Left alone, the comment would describe deleted code — exactly the documentation drift this
   workstream exists to remove (cf. task 203).

**Rewrite that doc comment** so it no longer names `MergedProject`, `merged.rs`, or
`/merged-projects`. Suggested replacement (adjust freely, but do not reintroduce those names):

```rust
//! Local projects with display enrichment: task counts, last attempt, and GitHub counts.
//!
//! The response deliberately carries no node/merge fields — this node serves local projects
//! only; hive-side project data lives on the hive (see ADR-0014).
```

**G2 — every other surviving reference is already inside `files:`. Verified pre-dispatch:**
`git grep -ln 'MergedProject\|MergedProjectsResponse\|NodeLocation\|merged-projects' -- crates frontend shared`
returns exactly `generate_types.rs`, `handlers/merged.rs`, `handlers/with_stats.rs` (G1),
`projects/mod.rs`, `projects/types.rs`, `frontend/src/lib/api/projects.ts`, `shared/types.ts`. All are
in `files:`. **If your grep finds a file NOT in that list, the STOP trigger is real — report it.**

**G3 — STOP triggers pre-checked:**
- `handlers/core.rs` contains NO reference to `MergedProject`/`NodeLocation` — leave it untouched
  (it is in `files:` only as a precaution).
- `TaskCounts` is used by `ProjectWithStats` (`types.rs:208`) and `with_stats.rs:16,44` — it will NOT
  become unused. If it does, task 301 is broken; STOP.
- `NodeLocation`'s only references are `generate_types.rs:31`, `projects/mod.rs:7`,
  `types.rs:130` (the `MergedProject.nodes` field) and its own declaration at `types.rs:151` — all
  removed by this task, as the contract states.
- Task 302 IS `passed` (board test green), so the "do not run" precondition is satisfied.

**G4 — `types.rs:88-110` (`impl From<Project> for RemoteNodeProject`) MUST SURVIVE.** It sits directly
above the structs you are deleting and constructs same-named fields (`node_name`, `node_status`,
`node_public_url`), which makes it easy to delete by accident. It backs `UnifiedProject::Remote`.

## Change

### 1. Delete the handler

- Delete `crates/server/src/routes/projects/handlers/merged.rs` (`git rm`).
- `crates/server/src/routes/projects/handlers/mod.rs`: delete the `mod merged;` declaration and
  the `pub use merged::get_merged_projects;` line.

### 2. Delete the route — `crates/server/src/routes/projects/mod.rs`

- **Anchor:** the final router expression (~line 146-149)
- **Before:**
```rust
    Router::new()
        .nest("/projects", projects_router)
        .route("/merged-projects", get(get_merged_projects))
}
```
- **After:**
```rust
    Router::new().nest("/projects", projects_router)
}
```

Also remove `get_merged_projects` from the handler import list, and `MergedProject`,
`MergedProjectsResponse`, and `NodeLocation` from the `pub use types::{...}` re-export block.

### 3. Delete the types — `crates/server/src/routes/projects/types.rs`

- Delete `pub struct MergedProject` (line 113-146, doc comment on 112) .
- Delete `pub struct NodeLocation` (line 150-164, doc comment on 149) — decomposition verified
  its only references are `MergedProject.nodes`, the `generate_types.rs` decl, and the
  `projects/mod.rs` re-export, all removed by this task.
- Delete `pub struct MergedProjectsResponse` (line 176-179, doc comment on 175).

> **There is NO `impl From<...> for NodeLocation`.** The `impl From<Project> for
> RemoteNodeProject` block at lines 88-110 looks adjacent and constructs `node_name` /
> `node_status` / `node_public_url` fields with the same names — it is a DIFFERENT type that
> must SURVIVE (`RemoteNodeProject` backs `UnifiedProject::Remote`). Do not touch lines 88-110.
> Likewise leave `TaskCounts` (line ~168) alone: task 301's `ProjectWithStats` uses it.

### 4. Deregister the ts-rs exports — `crates/server/src/bin/generate_types.rs`

Delete the `MergedProject::decl()`, `MergedProjectsResponse::decl()`, and `NodeLocation::decl()`
lines (~lines 30-33).

### 5. Delete the client method — `frontend/src/lib/api/projects.ts`

- **Anchor:** line 106-109
- **Before:**
```typescript
  getMerged: async (): Promise<MergedProjectsResponse> => {
    const response = await makeRequest('/api/merged-projects');
    return handleApiResponse<MergedProjectsResponse>(response);
  },
```
- **After:** (deleted). Remove `MergedProjectsResponse` from this file's type imports.

### 6. Regenerate types

```bash
npm run generate-types
```

## Allowed moves

- Only the files in `files:`. `core.rs` is listed **only** in case it re-exports or references
  `MergedProject`; if it does not, leave it untouched.

## STOP triggers

- If ANY reference to `MergedProject`, `MergedProjectsResponse`, or `NodeLocation` survives in a
  file NOT listed in `files:` — STOP and report the file. Do not edit it.
- If `TaskCounts` becomes unused — it should NOT, because task 301's `ProjectWithStats` uses it.
  If it does, task 301 is incomplete; STOP.
- If you are about to delete anything between lines 88 and 110 of `types.rs` — STOP. That is
  `impl From<Project> for RemoteNodeProject`, which survives.
- If task 302 is not `passed`, do not run this task — the board would break.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

npm run generate-types:check
# Expected: passes

git grep -n 'MergedProject\|MergedProjectsResponse\|NodeLocation\|merged-projects' -- crates frontend shared
# Expected: NO output
# (docs/ and dev-docs/ legitimately retain the term — see the note above)

cd frontend && npx tsc --noEmit && npm run lint && npx vitest run
# Expected: all clean/green

# With the dev server running:
curl -s -o /dev/null -w '%{http_code} %{content_type}\n' "http://127.0.0.1:${PORT}/api/merged-projects"
# Expected: text/html — the SPA fallback, proving the route is GONE.
# NOT 404: the outer catch-all `.route("/{*path}", ...)` (crates/server/src/routes/mod.rs:76)
# serves index.html with StatusCode::OK (frontend.rs:40-43) for any unmatched /api path.
# Asserting 404 here would FAIL even though the deletion succeeded.
curl -s -o /dev/null -w '%{http_code} %{content_type}\n' "http://127.0.0.1:${PORT}/api/projects/with-stats"
# Expected: 200 application/json
```

## Done when

- `/api/merged-projects` returns the SPA fallback (`text/html`), proving it is gone;
  `/api/projects/with-stats` returns `200 application/json`.
- No `MergedProject`, `MergedProjectsResponse`, `NodeLocation`, or `merged-projects` reference
  survives in `crates/`, `frontend/`, or `shared/types.ts`.
- Rust clippy, `generate-types:check`, and the full frontend gates are green.
