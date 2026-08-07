---
id: "601"
phase: 6
title: "Project auto_breakdown_enabled: migration + model + typegen"
status: ready
depends_on: ["103"]
parallel: false
conflicts_with: []
files:
  - "crates/db/migrations/20260807000100_add_project_auto_breakdown.sql"
  - "crates/db/src/models/project/mod.rs"
  - "crates/db/src/models/project/queries.rs"
  - "crates/db/src/models/project/stats.rs"
  - "crates/db/src/models/project/github.rs"
  - "crates/db/src/models/project/sync.rs"
  - "crates/server/src/routes/projects/handlers/core.rs"
  - "crates/server/src/routes/tasks/handlers/streams.rs"
  - "crates/server/src/bin/generate_types.rs"
  - "shared/types.ts"
irreversible: false
scope_test: "crates/db"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
In the project model's existing test module: create a project, assert auto_breakdown_enabled defaults to false; update it to true via the UpdateProject path; re-fetch and assert true.


## Change
**File:** new migration — exact content:
```sql
-- P3 auto-trigger opt-in (default OFF; SC5 requires unchanged behaviour when disabled).
ALTER TABLE projects ADD COLUMN auto_breakdown_enabled INTEGER NOT NULL DEFAULT 0;
```
**File:** crates/db/src/models/project/mod.rs — add `pub auto_breakdown_enabled: bool,` to Project (next to the script fields) and `pub auto_breakdown_enabled: Option<bool>,` to UpdateProject (sibling field: parallel_setup_script at line ~96 — mirror its Option<bool> handling end-to-end).
**Files:** crates/db/src/models/project/queries.rs, stats.rs, github.rs, sync.rs, crates/server/src/routes/projects/handlers/core.rs, crates/server/src/routes/tasks/handlers/streams.rs — thread the new column through EVERY Project materialization site exactly the way parallel_setup_script is threaded (tournament R1 F4/F-codex9 site inventory, verified 2026-08-07: query_as! column lists in queries.rs/github.rs/sync.rs, the full struct literal in stats.rs:62-84, and the Project literals/destructurings in the two server files; `grep -rn parallel_setup_script crates/` enumerates them; cargo check enumerates any miss).
**Files:** generate_types.rs untouched unless Project/UpdateProject decl lines are absent (they exist — verify only); shared/types.ts regenerated via npm run generate-types, committed verbatim, and `npm run generate-types:check` run to exit 0 as part of THIS task's gate (a stale generated file must fail here, not in 701; CodeRabbit PR470).


## Allowed moves
The migration, the two struct fields, the mechanical threading through the listed materialization sites mirroring parallel_setup_script, and the regenerated types. Nothing else — no handler behaviour changes, no UI work (602/603).


## STOP triggers
cargo check reveals a Project materialization site OUTSIDE the listed files (STOP and report the full list — amend the plan, do not silently widen); CreateProject also carries script fields in a way that forces a decision (record ledger entry, default to NOT adding the field to CreateProject).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 601` exits 0
