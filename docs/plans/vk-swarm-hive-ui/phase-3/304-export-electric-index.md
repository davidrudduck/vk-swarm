---
id: "304"
phase: 3
title: Export 3 new collections + types from electric/index.ts
status: ready
depends_on: ["301", "302", "303"]
parallel: false
conflicts_with: []
files:
  - frontend/src/lib/electric/index.ts
  - remote-frontend/src/lib/electric/index.ts
irreversible: false
scope_test: "N/A — no test file. Verified via tsc + downstream import in 305."
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)

N/A — wiring-only change. The gate is `cd remote-frontend && npx tsc --noEmit` (the new exports must be consumable from `305-tasks-board-page`'s `@/lib/electric` import, which resolves to `remote-frontend/src/lib/electric/`). If any of 301/302/303 are missing their symbols, tsc fails when 305 imports them.

## Change

### File: `remote-frontend/src/lib/electric/index.ts`

**Sibling alignment:** Read `remote-frontend/src/lib/electric/index.ts` (task 300's copy) AND `frontend/src/lib/electric/index.ts` (read-only; SC4). The existing `Collection exports` block lists each `createXxxCollection` factory + each `ElectricXxx` type + `ElectricCollectionConfig`. The new exports MUST follow the same grouping. Justify any divergence in the decisions ledger. Do NOT edit the node frontend's copy.

**Before (lines 32-41):**
```ts
// Collection exports
export {
  createNodesCollection,
  createProjectsCollection,
  createNodeProjectsCollection,
  type ElectricNode,
  type ElectricProject,
  type ElectricNodeProject,
  type ElectricCollectionConfig,
} from './collections';
```

**After:**
```ts
// Collection exports
export {
  createNodesCollection,
  createProjectsCollection,
  createNodeProjectsCollection,
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  type ElectricNode,
  type ElectricProject,
  type ElectricNodeProject,
  type ElectricTaskAssignment,
  type ElectricTaskOutputLog,
  type ElectricTaskProgressEvent,
  type ElectricCollectionConfig,
} from './collections';
```

## Allowed moves
- Add the 3 new factory exports + 3 new type exports to the existing `Collection exports` block in `remote-frontend/src/lib/electric/index.ts`.
- Read-only reference to `frontend/src/lib/electric/index.ts` (SC4 — do NOT edit it).
- No other file.

## STOP triggers
- Any of `createTaskAssignmentsCollection` / `createTaskOutputLogsCollection` / `createTaskProgressEventsCollection` are absent from `remote-frontend/src/lib/electric/collections.ts` (301/302/303 not done) — HALT; this task depends on all three.

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx tsc --noEmit
cd frontend && npx tsc --noEmit   # SC4 — node frontend untouched
```
Both exit 0. The 3 new factories + types are importable from `@/lib/electric` in the hive frontend (verified by 305's imports compiling).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx tsc --noEmit" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 304` exits 0