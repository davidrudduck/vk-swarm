---
id: "503"
phase: 5
title: "Card action (both dropdown branches) + proposed badge + i18n keys (en/ja/ko/es)"
status: ready
depends_on: ["502"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/components/ui/actions-dropdown.tsx"
  - "frontend/src/components/tasks/TaskCard.tsx"
  - "frontend/src/i18n/locales/en/tasks.json"
  - "frontend/src/i18n/locales/ja/tasks.json"
  - "frontend/src/i18n/locales/ko/tasks.json"
  - "frontend/src/i18n/locales/es/tasks.json"
irreversible: false
scope_test: "frontend/src/components/tasks"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
Extend the existing TaskCard test file (or create TaskCard.breakdown.test.tsx if none): with a draft proposal present (mock useBreakdownProposal), the card shows the proposed-subtasks badge; the actions dropdown contains the Break down item which shows BreakdownReviewDialog on click. Assert the menu item exists in BOTH renders if the test harness can drive the mobile branch; otherwise cover desktop and record the mobile-branch gap in the ledger.


## Change
**File:** actions-dropdown.tsx — TRAP (prior post-mortem): this file has TWO independent action lists — the desktop <DropdownMenu> tree (~line 648+) and the mobile bottom-sheet (portal, ~394-625). Add a 'Break down' item to BOTH, adjacent to the existing 'create subtask' action in each branch, opening BreakdownReviewDialog via its defineModal .show({taskId, projectId}) (trigger first via useBreakdownMutations.trigger when no proposal exists, then show; when a draft exists, just show). Label t('tasks:breakdown.action').
**File:** TaskCard.tsx — Anchor: the badge cluster at ~239-247 (status/attempt badges). Add a badge rendered when useBreakdownProposal(task.id) returns a draft proposal: label t('tasks:breakdown.proposedBadge'). Follow the existing badge component idiom.
**Files:** the four locale tasks.json — add under a new top-level `breakdown` object:
en: { "action": "Break down", "proposedBadge": "Proposed subtasks", "dialog": { "title": "Review proposed subtasks", "accept": "Accept", "discard": "Discard", "retry": "Retry", "running": "Breaking down…", "failed": "Breakdown failed", "empty": "No subtasks proposed yet" } }
ja: { "action": "タスクを分解", "proposedBadge": "提案されたサブタスク", "dialog": { "title": "提案されたサブタスクの確認", "accept": "承認", "discard": "破棄", "retry": "再試行", "running": "分解中…", "failed": "分解に失敗しました", "empty": "まだサブタスクの提案はありません" } }
ko: { "action": "작업 분해", "proposedBadge": "제안된 하위 작업", "dialog": { "title": "제안된 하위 작업 검토", "accept": "수락", "discard": "폐기", "retry": "다시 시도", "running": "분해 중…", "failed": "분해 실패", "empty": "아직 제안된 하위 작업이 없습니다" } }
es: { "action": "Desglosar", "proposedBadge": "Subtareas propuestas", "dialog": { "title": "Revisar subtareas propuestas", "accept": "Aceptar", "discard": "Descartar", "retry": "Reintentar", "running": "Desglosando…", "failed": "El desglose falló", "empty": "Aún no hay subtareas propuestas" } }


## Allowed moves
The two component edits at the stated anchors (both dropdown branches — omitting the mobile branch is a STOP-level violation) and the four locale additions. No refactors of the dropdown.


## STOP triggers
actions-dropdown structure has diverged from the two-branch shape researched; TaskCard badge cluster moved; locale files use a different nesting for feature keys (mirror whatever `rebase.dialog.*` uses).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 503` exits 0
