---
id: "103"
phase: 1
title: "Register breakdown types in generate_types.rs and regenerate shared/types.ts"
status: ready
depends_on: ["102"]
parallel: false
conflicts_with: []
files:
  - "crates/server/src/bin/generate_types.rs"
  - "shared/types.ts"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — covered by existing tests: `npm run generate-types:check` fails while shared/types.ts is stale; it is the gate for this task.


## Change
**File:** crates/server/src/bin/generate_types.rs
**Anchor:** the Vec<String> of `::decl()` calls; the existing task entries at lines ~67-79 (db::models::task::Task::decl() etc.).
Add, adjacent to the task entries:
```rust
        db::models::task_breakdown::TaskBreakdownProposal::decl(),
        db::models::task_breakdown::TaskBreakdownProposalItem::decl(),
        db::models::task_breakdown::BreakdownStatus::decl(),
        db::models::task_breakdown::UpsertProposalItems::decl(),
        db::models::task_breakdown::ProposalItemInput::decl(),
        db::models::task_breakdown::TaskDependency::decl(),
```
**File:** shared/types.ts — regenerated output only: run `npm run generate-types`; commit the resulting diff verbatim. DO NOT hand-edit.


## Allowed moves
Only the six decl() lines in generate_types.rs and the regenerated shared/types.ts. Nothing else.


## STOP triggers
generate-types emits diffs to types unrelated to task_breakdown (stale baseline — STOP and report); tsc in frontend breaks on the regenerated file.


## Manual verification (record in decisions-ledger)
Record in decisions-ledger: `npm run generate-types:check` exit 0 after regeneration; `cd frontend && npx tsc --noEmit` exit 0.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 103` exits 0
