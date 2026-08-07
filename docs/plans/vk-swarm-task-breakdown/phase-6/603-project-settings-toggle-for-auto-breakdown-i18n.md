---
id: "603"
phase: 6
title: "Project settings toggle for auto breakdown (+ i18n)"
status: ready
depends_on: ["601"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/pages/settings/ProjectSettings.tsx"
  - "frontend/src/i18n/locales/en/settings.json"
  - "frontend/src/i18n/locales/ja/settings.json"
  - "frontend/src/i18n/locales/ko/settings.json"
  - "frontend/src/i18n/locales/es/settings.json"
irreversible: false
scope_test: "frontend/src/pages/settings"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
Extend the ProjectSettings test file if one exists (else create a colocated test): load a project with auto_breakdown_enabled=false, assert the checkbox renders unchecked; toggle it and save; assert the UpdateProject payload includes auto_breakdown_enabled: true. Mirror however the parallel_setup_script checkbox is tested; if it has no test, model on the nearest tested field and note the gap in the ledger.


## Change
**File:** frontend/src/pages/settings/ProjectSettings.tsx (tournament R1 F5/F-codex10: the real editable state + UpdateProject payload + parallel_setup_script checkbox live HERE at :36-55, :216-224, :431-445 under useTranslation('settings') at :61 — NOT in ProjectFormFields.tsx, which serves the create dialog only). Anchor: the parallel_setup_script checkbox block (:431-445) and its threading through ProjectFormState / projectToFormState / the save payload (:36-55, :216-224). Add an identical field for auto_breakdown_enabled with label t('settings:projects.autoBreakdown.label') and helper t('settings:projects.autoBreakdown.help') — nest the keys wherever the sibling checkbox's keys live in settings.json (mirror its exact namespace path; adjust the t() calls to match).
**Files:** four settings.json locales — add (at the sibling key's nesting level):
en: { "autoBreakdown": { "label": "Auto-breakdown new tasks", "help": "When on, new tasks with a description get AI-proposed subtasks for your review. Nothing is created without acceptance." } }
ja: { "autoBreakdown": { "label": "新規タスクを自動分解", "help": "有効にすると、説明付きの新規タスクにAIがサブタスクを提案します。承認するまで何も作成されません。" } }
ko: { "autoBreakdown": { "label": "새 작업 자동 분해", "help": "켜면 설명이 있는 새 작업에 대해 AI가 하위 작업을 제안합니다. 수락 전에는 아무것도 생성되지 않습니다." } }
es: { "autoBreakdown": { "label": "Desglose automático de tareas nuevas", "help": "Al activarlo, las tareas nuevas con descripción reciben subtareas propuestas por IA para tu revisión. No se crea nada sin aceptación." } }


## Allowed moves
The one field block (mirroring the parallel_setup_script checkbox's exact idiom incl. form-state threading) and the locale additions. No form refactors.


## STOP triggers
The parallel_setup_script checkbox is absent from ProjectSettings.tsx at the researched anchors (re-locate before proceeding; if it lives in a different file, STOP — files list must be amended); update payload shape rejects the new key (601 typegen gap).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 603` exits 0
