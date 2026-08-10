---
id: "601"
phase: 6
title: "Project auto_breakdown_enabled: migration + model + typegen"
status: passed
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
  - "crates/db/.sqlx/query-188d4124d3b360b111a7d5fa038a736f0b5c818fb9d99203683a610e2699e684.json"
  - "crates/db/.sqlx/query-22d49a9268477c822e031b9c18ca278283cbfc2843b8111a7c101d6cc7cf4c91.json"
  - "crates/db/.sqlx/query-426fd871dfadc5e927637343c30dac801afee4e8d81076d734f477e8ab0054ba.json"
  - "crates/db/.sqlx/query-5246b2001c06d312446a1be5fc3c559d8c6be15445c91fbd8c1013e95b5ca4d2.json"
  - "crates/db/.sqlx/query-52cb1f53a47a79f90738a36a72329eb06b21608b9a08006b2dd356b4fd345c08.json"
  - "crates/db/.sqlx/query-64d5489956117c27cfeb27680a3e7c824e495ceb86cdf3309c4d908637ebb481.json"
  - "crates/db/.sqlx/query-69b7da2baeaae86174972a29e0917a9ecf13ac6cc358c3b0d0a6abf764aa8bc6.json"
  - "crates/db/.sqlx/query-6c446efaf402c3cff9246bb199192640b7775df830774505e07ebff61afb6f02.json"
  - "crates/db/.sqlx/query-82edb464e8095c7cc5bed649c71171acd6278c2f30a3adcd963540449a702518.json"
  - "crates/db/.sqlx/query-99af7434117d6694e1008112d6d04df4042a5aea9f0083956bed25c39a014594.json"
  - "crates/db/.sqlx/query-9ee8ed41875c5acac9c4865950952268e6e4b68aff53e7efeb57520bf1c280d0.json"
  - "crates/db/.sqlx/query-a30f13ff548e884125c3fc7c87b44cb1b3bb3c29f90bfc54e26fefc6d5e919c4.json"
  - "crates/db/.sqlx/query-afaeb58539c13d77124ce8fcb27ccb14d1b4cd6dc858ac545b429f9fcb48b072.json"
  - "crates/db/.sqlx/query-b46ea36ab332ac2b19f6fff06c7dc6934e362e5d4c66cf9b67e72d16389bd3d7.json"
  - "crates/db/.sqlx/query-b585c64fc315b2dacf75496446c7d46ae8285bc71b99768fad35b230b568b358.json"
  - "crates/db/.sqlx/query-bc754f652a9f61425d8872d2ac982d89308c33f64d557618f68ecb4ddc2e794e.json"
  - "crates/db/.sqlx/query-c398ed006d7286e8fdbf17b646e306a2f6d9038a63242cc65661846d555920f6.json"
  - "crates/db/.sqlx/query-df585a83d980b20eeba1a9e63eb497ae25be7200bebb0e6a6bce491d2b8fe2fd.json"
  - "crates/db/.sqlx/query-e4b71252c9f112d9f9e09148975190226f99140d1f981f5261582bb3088f7339.json"
  - "frontend/src/components/tasks/TaskDetails/preview/NoServerContent.tsx"
  - "frontend/src/pages/settings/ProjectSettings.tsx"
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
