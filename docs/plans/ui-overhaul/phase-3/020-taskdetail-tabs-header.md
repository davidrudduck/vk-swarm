---
id: "020"
phase: 3
title: Task-detail view switcher → labeled Tabs (Diff/Logs/Attempts) + StatusBadge dot
status: ready
depends_on: ["014"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/panels/AttemptHeaderActions.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC18]
---
## Failing test (write first)
N/A — covered by manual verification (greppable assertions + browser smoke-test below; the
chrome change is presentational and a unit test cannot cheaply assert the rendered Tabs).

## Change

### File: `frontend/src/components/panels/AttemptHeaderActions.tsx`

This component is the **right-side actions cluster** of the desktop task-detail header
(`NewCardHeader`, rendered in `frontend/src/pages/ProjectTasks.tsx` ~860–915). The task **title**
already lives in the parent's `<Breadcrumb>` (`truncateTitle(selectedTask?.title)`,
ProjectTasks.tsx ~887–898), and a close `X` already exists in this fragment (~lines 154–156). See
`## STOP triggers` / ledger note for what this task does **not** cover (the standalone
`text-lg font-semibold` title and the labels portion of the badges row).

Per ADR-0006 / SC18, change the IN-PANEL chrome (do NOT add a drawer/sheet; the panel stays
resizable). Two edits:

**Edit 1 — Replace the icon-only `ToggleGroup` (lines ~57–149) with a labeled `<Tabs>`.**

The Tabs import path is `@/components/ui/tabs` (named exports `Tabs, TabsList, TabsTrigger`;
confirmed present). Three labeled tabs with this mode mapping (preserving the existing
`mode`/`onModeChange` contract — `value` is the current mode, selecting a tab calls
`onModeChange(newMode)`):

- **Diff** → sets mode `'diffs'`
- **Logs** → sets mode `'terminal'`
- **Attempts** → sets mode `null` (resets to attempt-history)

- Before (lines ~57–149 — the `!isMobile && typeof mode !== 'undefined' && onModeChange` guarded
  `TooltipProvider` > `ToggleGroup` block, the icon-only items for `preview`/`diffs`/`files`/
  `terminal`/`processes`):
```tsx
      {!isMobile && typeof mode !== 'undefined' && onModeChange && (
        <TooltipProvider>
          <ToggleGroup
            type="single"
            value={mode ?? ''}
            onValueChange={(v) => {
              const newMode = (v as LayoutMode) || null;
              onModeChange(newMode);
            }}
            className="inline-flex gap-4"
            aria-label="Layout mode"
          >
            <Tooltip>
              <TooltipTrigger asChild>
                <ToggleGroupItem
                  value="preview"
                  aria-label="Preview"
                  active={mode === 'preview'}
                >
                  <Eye className="h-4 w-4" />
                </ToggleGroupItem>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {t('attemptHeaderActions.preview')}
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <ToggleGroupItem
                  value="diffs"
                  aria-label="Diffs"
                  active={mode === 'diffs'}
                >
                  <FileDiff className="h-4 w-4" />
                </ToggleGroupItem>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {t('attemptHeaderActions.diffs')}
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <ToggleGroupItem
                  value="files"
                  aria-label="Files"
                  active={mode === 'files'}
                >
                  <FolderTree className="h-4 w-4" />
                </ToggleGroupItem>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {t('attemptHeaderActions.files', { defaultValue: 'Files' })}
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <ToggleGroupItem
                  value="terminal"
                  aria-label="Terminal"
                  active={mode === 'terminal'}
                >
                  <Terminal className="h-4 w-4" />
                </ToggleGroupItem>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {t('attemptHeaderActions.terminal', {
                  defaultValue: 'Terminal',
                })}
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <ToggleGroupItem
                  value="processes"
                  aria-label="Processes"
                  active={mode === 'processes'}
                >
                  <Cog className="h-4 w-4" />
                </ToggleGroupItem>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {t('attemptHeaderActions.processes', {
                  defaultValue: 'Processes',
                })}
              </TooltipContent>
            </Tooltip>
          </ToggleGroup>
        </TooltipProvider>
      )}
```
- After (labeled Tabs; keep the same `!isMobile && typeof mode !== 'undefined' && onModeChange`
  guard so mobile behaviour is unchanged — mobile already uses `MobileViewModeSheet`. The current
  mode is mapped to a tab `value`; modes with no tab fall back to `'attempts'` so the Tabs
  component always has a defined value):
```tsx
      {!isMobile && typeof mode !== 'undefined' && onModeChange && (
        <Tabs
          value={
            mode === 'diffs'
              ? 'diff'
              : mode === 'terminal'
                ? 'logs'
                : 'attempts'
          }
          onValueChange={(v) => {
            onModeChange(
              v === 'diff' ? 'diffs' : v === 'logs' ? 'terminal' : null
            );
          }}
          aria-label="Layout mode"
        >
          <TabsList>
            <TabsTrigger value="diff">
              {t('attemptHeaderActions.diffs')}
            </TabsTrigger>
            <TabsTrigger value="logs">
              {t('attemptHeaderActions.terminal', { defaultValue: 'Logs' })}
            </TabsTrigger>
            <TabsTrigger value="attempts">
              {t('attemptHeaderActions.attempts', { defaultValue: 'Attempts' })}
            </TabsTrigger>
          </TabsList>
        </Tabs>
      )}
```

Fix the imports to match: remove the now-unused `ToggleGroup`/`ToggleGroupItem` import (line 4) and
the `Tooltip*` imports (lines 5–10) **only if** no longer referenced after this edit (they are not
used elsewhere in this file — confirm by grep before deleting). Remove the now-unused lucide icons
`Eye, FileDiff, FolderTree, Terminal, Cog` from line 2 (keep `X`). Add
`import { Tabs, TabsList, TabsTrigger } from '../ui/tabs';`.

**Edit 2 — Add a `StatusBadge` dot to the actions cluster.**

`StatusBadge` is the NEW component from task 014: named export from `@/components/common/StatusBadge`,
props `{ status: TaskStatus; showLabel?: boolean; className?: string }`. The task status is already
available in this component's props as `task.status` (`TaskWithAttemptStatus` extends `Task`, which
has `status: TaskStatus`). Add the dot at the start of the returned fragment, before the connection
badge:

- Before (the opening of the returned fragment, ~lines 48–56):
```tsx
  return (
    <>
      {/* Connection status badge for remote tasks */}
      {showConnectionBadge && (
```
- After:
```tsx
  return (
    <>
      <StatusBadge status={task.status} className="mr-1" />
      {/* Connection status badge for remote tasks */}
      {showConnectionBadge && (
```
Add `import { StatusBadge } from '@/components/common/StatusBadge';` alongside the existing
`ConnectionStatusBadge` import (line 15).

## Allowed moves
- ONLY in `frontend/src/components/panels/AttemptHeaderActions.tsx`: replace the icon-only
  `ToggleGroup` block with the labeled `Tabs` block; add the `StatusBadge` dot; adjust the imports
  to match (drop unused `ToggleGroup`/`ToggleGroupItem`/`Tooltip*`/icon imports, add `Tabs*` and
  `StatusBadge`). Do not touch `ProjectTasks.tsx`, `TasksLayout.tsx`, or any other file. Do not add
  a standalone `text-lg font-semibold` title or a labels badges row (see STOP triggers — deferred).

## STOP triggers
- **Scope boundary (intentional, not a bug): this task does NOT cover the SC18 standalone header
  title or the full badges row.** The task title already renders in the parent `NewCardHeader`
  `<Breadcrumb>` (ProjectTasks.tsx ~887–898); adding a `text-lg font-semibold` title here would
  duplicate it. The badges row's **label** badges require label data that is NOT on
  `TaskWithAttemptStatus` props — out of scope without parent prop-threading. Node data
  (`task.source_node_name`) IS reachable but a node badge is deferred to keep this task surgical.
  If the executor believes the title/labels MUST land here, STOP and reconcile — do not fabricate
  label data or duplicate the breadcrumb title. Record the deferred header-title + labels/node
  badges-row threading in the decisions-ledger as a follow-up.
- **Dropped modes — do NOT silently drop `preview`/`files`/`processes`.** The new 3-tab set covers
  only `diffs`/`terminal`/`null`. `preview` and `files` remain reachable via the keyboard cycle in
  ProjectTasks.tsx (~line 558: `const order: LayoutMode[] = [null, 'preview', 'diffs', 'files'];`).
  **`processes` is in neither the new tabs nor that cycle** — after this change it has no
  desktop affordance in this header. Before committing, EITHER confirm `processes` has another
  surface (re-grep `mode === 'processes'` across `frontend/src`; the panel still renders at
  ProjectTasks.tsx ~983) OR halt and record in the ledger that `processes` is now only
  programmatically reachable, for explicit sign-off. Do not delete the `processes` rendering branch.
- The `ToggleGroup` block body differs materially from the Before text (re-grep
  `value={mode ?? ''}`; if absent, halt — the file changed since decompose).
- `@/components/ui/tabs` does not export `Tabs`/`TabsList`/`TabsTrigger` (halt; reconcile import).
- `@/components/common/StatusBadge` does not exist or lacks the `{ status }` prop (halt — task 014
  not applied; this task `depends_on: ["014"]`).

## Manual verification (record in decisions-ledger)
- `grep -n '<Tabs' frontend/src/components/panels/AttemptHeaderActions.tsx` → match.
- `grep -nE "attemptHeaderActions\.(diffs|terminal|attempts)" frontend/src/components/panels/AttemptHeaderActions.tsx`
  → the three tab labels (Diff / Logs / Attempts) present.
- `grep -n '<StatusBadge' frontend/src/components/panels/AttemptHeaderActions.tsx` → match.
- `grep -n 'ToggleGroup' frontend/src/components/panels/AttemptHeaderActions.tsx` → no match
  (old icon switcher removed).
- `cd frontend && npx tsc --noEmit` → passes.
- Browser: open a task detail panel; confirm the Diff / Logs / Attempts labeled tabs switch the aux
  panel (Diff→diffs, Logs→terminal, Attempts→attempt history) and the status dot renders. Confirm
  no console errors.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 020` exits 0
