---
id: "201"
phase: 2
title: "Add ExecutionProcessRunReason::Breakdown variant"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/execution_process/mod.rs"
  - "shared/types.ts"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — covered by existing tests: cargo check --workspace proves no exhaustive match breaks (survey 2026-08-07: all run_reason matches are matches!/guarded, none exhaustive); generate-types:check pins the TS union.


## Change
**File:** crates/db/src/models/execution_process/mod.rs
**Anchor:** `pub enum ExecutionProcessRunReason` at line ~55 (variants SetupScript, CleanupScript, CodingAgent, DevServer).
**Before:**
```rust
    CodingAgent,
    DevServer,
}
```
**After:**
```rust
    CodingAgent,
    DevServer,
    Breakdown,
}
```
(The enum's existing serde rename_all governs the wire value; do not add per-variant attributes.)
**File:** shared/types.ts — regenerated via `npm run generate-types` (the ExecutionProcessRunReason union gains 'breakdown'). Commit verbatim.


## Allowed moves
Exactly one variant line added; the regenerated shared/types.ts. If `cargo check --workspace` surfaces an exhaustive-match error in ANY other file, STOP (the plan missed a site) — do not edit unlisted files.


## STOP triggers
Exhaustive-match compile error outside the listed files; the enum carries per-variant serde/strum attributes implying a different addition pattern; frontend tsc breaks on the widened union.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 201` exits 0
