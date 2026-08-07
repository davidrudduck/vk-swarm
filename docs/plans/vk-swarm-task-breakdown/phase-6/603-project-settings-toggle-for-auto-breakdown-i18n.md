---
id: "603"
phase: 6
title: "Project settings toggle for auto breakdown (+ i18n)"
status: ready
depends_on: ["601"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/components/projects/ProjectFormFields.tsx"
  - "frontend/src/i18n/locales/en/projects.json"
  - "frontend/src/i18n/locales/ja/projects.json"
  - "frontend/src/i18n/locales/ko/projects.json"
  - "frontend/src/i18n/locales/es/projects.json"
irreversible: false
scope_test: "frontend/src/components/projects"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
Extend the project-form test file if one exists (else create ProjectFormFields.breakdown.test.tsx): the toggle renders, reflects auto_breakdown_enabled from the project, and flipping it includes auto_breakdown_enabled in the update payload. Mirror however parallel_setup_script's toggle is tested; if it has no test, model on the nearest tested boolean field and note the gap in the ledger.


## Change
**File:** ProjectFormFields.tsx — Anchor: the existing parallel_setup_script (or nearest boolean/checkbox) field block; add an identical field for auto_breakdown_enabled with label t('projects:autoBreakdown.label') and helper t('projects:autoBreakdown.help'), threaded through the same form state/update path.
**Files:** four projects.json locales — add:
en: { "autoBreakdown": { "label": "Auto-breakdown new tasks", "help": "When on, new tasks with a description get AI-proposed subtasks for your review. Nothing is created without acceptance." } }
ja: { "autoBreakdown": { "label": "新規タスクを自動分解", "help": "有効にすると、説明付きの新規タスクにAIがサブタスクを提案します。承認するまで何も作成されません。" } }
ko: { "autoBreakdown": { "label": "새 작업 자동 분해", "help": "켜면 설명이 있는 새 작업에 대해 AI가 하위 작업을 제안합니다. 수락 전에는 아무것도 생성되지 않습니다." } }
es: { "autoBreakdown": { "label": "Desglose automático de tareas nuevas", "help": "Al activarlo, las tareas nuevas con descripción reciben subtareas propuestas por IA para tu revisión. No se crea nada sin aceptación." } }


## Allowed moves
The one field block (mirroring the sibling field's exact idiom) and the locale additions. No form refactors.


## STOP triggers
ProjectFormFields.tsx has no boolean-field precedent to mirror (find where parallel_setup_script is edited in the UI and use THAT file instead only after confirming it — if it is a different file, STOP: files list must be amended); update payload shape rejects the new key (601 typegen gap).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 603` exits 0
