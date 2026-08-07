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
**File:** crates/db/src/models/project/queries.rs — extend the SELECT column lists and the update SQL exactly the way parallel_setup_script is threaded (read every site that names parallel_setup_script and mirror; sqlx query_as! will enumerate misses at compile time).
**Files:** generate_types.rs untouched unless Project/UpdateProject decl lines are absent (they exist — verify only); shared/types.ts regenerated via npm run generate-types, committed verbatim.


## Allowed moves
The migration, the two struct fields, the mechanical query-site threading mirroring parallel_setup_script, and the regenerated types. Nothing else — no handler/UI work (602/603).


## STOP triggers
parallel_setup_script threading touches files beyond the two listed model files (then STOP and report the full site list — the plan must be amended, not silently widened); CreateProject also carries script fields in a way that forces a decision (record ledger entry, default to NOT adding the field to CreateProject).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 601` exits 0
