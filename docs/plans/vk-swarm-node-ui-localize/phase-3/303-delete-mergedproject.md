---
id: "303"
phase: 3
title: "Delete MergedProject, NodeLocation, and the /api/merged-projects endpoint"
status: ready
depends_on: ["302"]
parallel: false
conflicts_with: ["301"]
files:
  - crates/server/src/routes/projects/handlers/merged.rs
  - crates/server/src/routes/projects/handlers/mod.rs
  - crates/server/src/routes/projects/types.rs
  - crates/server/src/routes/projects/mod.rs
  - crates/server/src/bin/generate_types.rs
  - crates/server/src/routes/projects/handlers/core.rs
  - frontend/src/lib/api/projects.ts
  - shared/types.ts
irreversible: true
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---

## Failing test (write first)

N/A — deletion. The proof is `cargo clippy -D warnings` + `tsc --noEmit` staying green with the
type gone, the scoped `git grep` over `crates/ frontend/ shared/`, and the 404 assertion in
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

- Delete `pub struct MergedProject` (~line 113-146) with its doc comment.
- Delete `pub struct MergedProjectsResponse` (~line 176-179) with its doc comment.
- Delete `pub struct NodeLocation` (~line 148-161) with its doc comment — decomposition verified
  its only references are `MergedProject.nodes`, the `generate_types.rs` decl, and the
  `projects/mod.rs` re-export, all removed by this task.
- If a `impl From<...> for NodeLocation` block exists (around line 90-110), delete it too, and
  check whether that leaves its source type unused — if it does, STOP rather than widening.

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
- If task 302 is not `passed`, do not run this task — the board would break.

## Manual verification (record in decisions-ledger)

```bash
cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

npm run generate-types:check
# Expected: passes

git grep -n 'MergedProject\|merged-projects' -- crates frontend shared
# Expected: NO output
# (docs/ and dev-docs/ legitimately retain the term — see the note above)

cd frontend && npx tsc --noEmit && npm run lint && npx vitest run
# Expected: all clean/green

# With the dev server running:
curl -s -o /dev/null -w '%{http_code}\n' "http://127.0.0.1:${PORT}/api/merged-projects"
# Expected: 404
curl -s -o /dev/null -w '%{http_code}\n' "http://127.0.0.1:${PORT}/api/projects/with-stats"
# Expected: 200
```

## Done when

- `/api/merged-projects` returns 404; `/api/projects/with-stats` returns 200.
- No `MergedProject`, `MergedProjectsResponse`, `NodeLocation`, or `merged-projects` reference
  survives in `crates/`, `frontend/`, or `shared/types.ts`.
- Rust clippy, `generate-types:check`, and the full frontend gates are green.
