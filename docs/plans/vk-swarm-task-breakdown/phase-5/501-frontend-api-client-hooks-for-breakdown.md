---
id: "501"
phase: 5
title: "Frontend api client + hooks for breakdown"
status: ready
depends_on: ["301"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/lib/api/breakdown.ts"
  - "frontend/src/lib/api/index.ts"
  - "frontend/src/hooks/useBreakdown.ts"
siblings: ["frontend/src/lib/api/tasks.ts","frontend/src/hooks/useTaskMutations.ts"]
irreversible: false
scope_test: "frontend/src/hooks"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
frontend/src/hooks/useBreakdown.test.ts (vitest, mirror the mocking style of the crate's existing hook tests — read useAvailableNodes.test.ts / useTaskMutations conventions first). Cases: useBreakdownProposal fetches GET /api/tasks/:id/breakdown and exposes proposal+items; useBreakdownMutations — ALL FIVE mutations (trigger, putItems, discard, retry, accept) invalidate ['breakdown', taskId] on success (each changes the cached proposal; CodeRabbit PR470); accept ADDITIONALLY invalidates ['tasks', projectId]. Strict-TS trap (prior ledgers): no unused imports, null-safe access on possibly-null proposal.


## Change
**File:** frontend/src/lib/api/breakdown.ts (new) — namespace object `breakdownApi` following the tasks.ts pattern (fetch + ApiResponse unwrap + typed returns using generated TaskBreakdownProposal/TaskBreakdownProposalItem/UpsertProposalItems/Task from shared/types): get(taskId), trigger(taskId), putItems(proposalId, payload), accept(proposalId): Promise<Task[]>, discard(proposalId), retry(proposalId), dependencies(taskId).
**File:** frontend/src/lib/api/index.ts — add the re-export line alongside tasks.
**File:** frontend/src/hooks/useBreakdown.ts (new) — `useBreakdownProposal(taskId)` (useQuery, key ['breakdown', taskId]) and `useBreakdownMutations(taskId, projectId, options?)` following the useTaskMutations option-callback conventions (on{Action}Success/Error per CLAUDE.md), with the invalidations stated in the failing test.


## Allowed moves
The three files only. No component work (502/503).


## STOP triggers
Generated types from 103 absent in shared/types.ts (typegen drift — STOP); the api/index.ts re-export pattern differs from researched.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 501` exits 0
