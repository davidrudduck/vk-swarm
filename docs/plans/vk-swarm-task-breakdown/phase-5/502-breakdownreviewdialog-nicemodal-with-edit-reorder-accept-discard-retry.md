---
id: "502"
phase: 5
title: "BreakdownReviewDialog (NiceModal) with edit/reorder/accept/discard/retry"
status: ready
depends_on: ["501"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/components/dialogs/tasks/BreakdownReviewDialog.tsx"
  - "frontend/src/components/dialogs/tasks/BreakdownReviewDialog.test.tsx"
siblings: ["frontend/src/components/dialogs/tasks/TaskFormSheet.tsx","frontend/src/components/dialogs/tasks/DeleteTaskConfirmationDialog.tsx"]
irreversible: false
scope_test: "frontend/src/components/dialogs/tasks"
allowed_change: create
covers_criteria: []
covers_tests: ["TS4"]
---
## Failing test (write first)
frontend/src/components/dialogs/tasks/BreakdownReviewDialog.test.tsx (vitest + testing-library, mirror the mocking approach of existing dialog/component tests). Cases: renders items with titles + dependency chips; edit title persists via putItems on save; delete item removes row; Accept calls breakdownApi.accept then closes; Discard calls discard; status 'failed' renders the localized error + Retry button wired to retry; status 'draft' with zero items and a live run renders the running state. i18n: assert keys resolve via the test i18n harness (no literal strings — eslint i18next rule).


## Change
**File:** BreakdownReviewDialog.tsx (new) — NiceModal.create + defineModal registration EXACTLY per TaskFormSheet.tsx (read it first; list its structural choices: useModal(), Dialog primitives from components/ui/dialog, mutation wiring via hooks not raw api). Props: { taskId: string; projectId: string }. Body: item list (title input, description textarea, sort drag or up/down buttons, dependency multi-select among sibling items by index, delete); footer: Discard (destructive), Accept (primary, disabled while items empty or a save is in flight); failed state banner with retry; running state (proposal exists, items empty, not failed) with spinner. All strings via useTranslation('tasks') under the `breakdown.` key namespace (keys land in 503 — use the keys, 503 adds locale files; within THIS task add the en keys ONLY if the test harness fails on missing keys, and note it in the ledger).
**File:** the colocated test per Failing test.


## Allowed moves
The two new files only. Uses hooks/api from 501; no changes to them.


## STOP triggers
defineModal/NiceModal pattern differs from researched; Dialog primitives require props not derivable from siblings; the drag/reorder primitive doesn't exist (fall back to up/down buttons — allowed, note in ledger).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 502` exits 0
