---
id: "020"
phase: 3
title: Task-detail header chrome (SC18) — labeled Tabs + StatusBadge + node/label badges
status: passed
depends_on: ["009", "014"]
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
chrome change is presentational and a unit test cannot cheaply assert the rendered Tabs/badges).

## Change

### File: `frontend/src/components/panels/AttemptHeaderActions.tsx`

This component is the **`actions` slot** of the desktop task-detail header (`NewCardHeader`, rendered
in `frontend/src/pages/ProjectTasks.tsx` ~869). The task **title** already lives in the parent's
`<Breadcrumb>` (`truncateTitle(selectedTask?.title)`, ProjectTasks.tsx ~882–913), so this task does
**not** add a duplicate title (see `## STOP triggers`). A close `X` already exists in this fragment
(~lines 154–156), and `ConnectionStatusBadge` is already imported (~line 15) and conditionally
rendered (~line 53).

**Placement reality (READ `frontend/src/components/ui/new-card.tsx` — confirmed):** `NewCardHeader`
renders its `actions` prop inside a single top-right inline flex row
(`<div className="flex items-center gap-4">{actions}</div>`, new-card.tsx:35), `items-center` on the
header. `AttemptHeaderActions` returns a **fragment** dropped into that row. There is therefore **no
"row below the header"** reachable from inside this component — a literal below-header band would
require editing `new-card.tsx` or `ProjectTasks.tsx`, both **out of scope** for this task's `files:`.
This task implements the SC18 badges as an **inline badges cluster within the actions row** (the only
placement achievable without a forbidden edit), wrapped in its own flex container so it reads as a
grouped unit. The deviation from a literal below-header band is documented in `## STOP triggers` and
the decisions-ledger; do NOT silently edit `new-card.tsx`/`ProjectTasks.tsx` to relocate it.

Per ADR-0006 / SC18, change the IN-PANEL chrome (do NOT add a drawer/sheet; the panel stays
resizable). Three edits.

**Edit 1 — Replace the icon-only `ToggleGroup` (lines ~57–149) with a labeled `<Tabs>`.**

The Tabs import path is `../ui/tabs` (named exports `Tabs, TabsList, TabsTrigger`; confirmed present
— `frontend/src/components/ui/tabs.tsx:53` exports `Tabs, TabsList, TabsTrigger, TabsContent`).
Three labeled tabs with this mode mapping (preserving the existing `mode`/`onModeChange` contract —
`value` reflects the current mode, selecting a tab calls `onModeChange(newMode)`):

- **Diff** → sets mode `'diffs'`
- **Logs** → sets mode `'terminal'`
- **Attempts** → sets mode `null` (resets to attempt-history)

Tab labels are **LITERAL English strings** with a `// TODO(i18n): vk-swarm-node-ui-localize` comment.
Do **NOT** reuse the existing `t('attemptHeaderActions.diffs')` / `.terminal` keys — those render
"Diffs"/"Terminal" (see `frontend/src/i18n/locales/en/tasks.json:331`), not the SC18 "Diff"/"Logs".

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
  mode is mapped to a tab `value`; modes with no tab fall back to `'attempts'` so the Tabs component
  always has a defined value):
```tsx
      {!isMobile && typeof mode !== 'undefined' && onModeChange && (
        // TODO(i18n): vk-swarm-node-ui-localize — Diff / Logs / Attempts are
        // literal English (the existing attemptHeaderActions.* keys render
        // "Diffs"/"Terminal", not the SC18 labels).
        <Tabs
          value={
            mode === 'diffs' ? 'diff' : mode === 'terminal' ? 'logs' : 'attempts'
          }
          onValueChange={(v) => {
            onModeChange(v === 'diff' ? 'diffs' : v === 'logs' ? 'terminal' : null);
          }}
          aria-label="Layout mode"
        >
          <TabsList>
            <TabsTrigger value="diff">Diff</TabsTrigger>
            <TabsTrigger value="logs">Logs</TabsTrigger>
            <TabsTrigger value="attempts">Attempts</TabsTrigger>
          </TabsList>
        </Tabs>
      )}
```

Fix the imports to match: remove the now-unused `ToggleGroup`/`ToggleGroupItem` import (line 4) and
the `Tooltip*` imports (lines 5–10) — neither is referenced elsewhere in this file after this edit
(confirm by grep before deleting). Remove the now-unused lucide icons `Eye, FileDiff, FolderTree,
Terminal, Cog` from line 2 (keep `X`). Add `import { Tabs, TabsList, TabsTrigger } from '../ui/tabs';`.

**`useTranslation` becomes unused — remove it.** After Edit 1 (literal tab labels) and Edits 2–3
(literal/no `t()` usage), `t` is referenced nowhere in this file, so leaving `useTranslation`
imported + `const { t } = useTranslation('tasks')` will fail `tsc --noEmit` (unused symbol). Remove
the `import { useTranslation } from 'react-i18next';` line (line 1) and the
`const { t } = useTranslation('tasks');` line (~line 34). **Re-grep `\bt(` in this file after editing
to confirm zero remaining `t(...)` call sites before deleting the import.**

**Edit 2 — Add a `StatusBadge` to a badges cluster, and a node badge.**

`StatusBadge` is the NEW component from task 014: named export from `@/components/common/StatusBadge`,
props `{ status: TaskStatus; showLabel?: boolean; className?: string }`. The task status is already
available as `task.status` (`TaskWithAttemptStatus` extends `Task`, which has `status: TaskStatus`).
`task.source_node_name` is also available on props. The node badge uses the shadcn `Badge` component
(`@/components/ui/badge`, variant `secondary` — confirmed present at `badge.tsx`), shown only when
`task.source_node_name` is present.

Insert a badges cluster at the **start of the returned fragment**, before the connection badge. SC18
specifies TWO status renderings (spec:96 "header renders `StatusBadge` dot" + spec:97 badges-row
"status outline+dot"), so render BOTH: a leading bare dot (`<StatusBadge status={...} />`, no label)
and the row outline status badge (`<StatusBadge status={...} showLabel className="…border…" />`). Both
live in the actions-slot cluster (the breadcrumb title is in `ProjectTasks.tsx`, out of scope — the
"header dot" therefore renders in this cluster, same placement constraint as the rest — see ledger).

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
      {/* SC18 chrome: header status dot + badges cluster (status outline+dot, node,
          labels). NewCardHeader renders the actions slot as a top-right inline row
          (new-card.tsx), so this is an inline cluster, not a literal below-header
          band, and the header dot sits here too — see ledger. */}
      <div className="flex items-center gap-1.5">
        {/* SC18:96 — header status dot (no label) */}
        <StatusBadge status={task.status} />
        {/* SC18:97 — row status badge: outline + dot + label */}
        <StatusBadge
          status={task.status}
          showLabel
          className="rounded-full border border-border px-2 py-0.5"
        />
        {task.source_node_name && (
          <Badge variant="secondary">{task.source_node_name}</Badge>
        )}
        {labels?.map((label) => (
          <LabelBadge
            key={label.id}
            label={label}
            variant="outline"
            size="sm"
          />
        ))}
      </div>
      {/* Connection status badge for remote tasks */}
      {showConnectionBadge && (
```
Add these imports alongside the existing `ConnectionStatusBadge` import (line 15):
```tsx
import { StatusBadge } from '@/components/common/StatusBadge';
import { Badge } from '@/components/ui/badge';
import { LabelBadge } from '@/components/labels/LabelBadge';
```

**Edit 3 — Source the label data via the `useTaskLabels` hook (same hook `TaskCard` uses).**

`TaskCard` (`frontend/src/components/tasks/TaskCard.tsx:97`) fetches labels with
`const { data: labels } = useTaskLabels(task.id, true);` from `@/hooks/useTaskLabels`. The hook
signature is `useTaskLabels(taskId: string | undefined, enabled = true)` and returns a TanStack
`useQuery` result whose `data` is `Label[] | undefined` (keyed by `['taskLabels', taskId]`). It needs
only `task.id` — no `projectId` — so it is fully sourceable from this component's props. (TaskCard
renders labels via `CompactLabelList`, which wraps `LabelBadge` in the SOLID variant; SC18 wants the
OUTLINE variant, so this task replicates the **hook**, not `CompactLabelList`, and renders each label
directly with `variant="outline"`.)

Add the hook call near the other hooks at the top of the component body (after
`useRemoteConnectionStatus`, ~line 40):

- Before (~lines 38–41):
```tsx
  const { status: connectionStatus } = useRemoteConnectionStatus(task, {
    enabled: Boolean(attempt) && task?.has_in_progress_attempt === true,
  });
```
- After:
```tsx
  const { status: connectionStatus } = useRemoteConnectionStatus(task, {
    enabled: Boolean(attempt) && task?.has_in_progress_attempt === true,
  });
  // Labels for the SC18 badges row (same hook TaskCard uses — keyed by task.id).
  const { data: labels } = useTaskLabels(task.id, true);
```
Add `import { useTaskLabels } from '@/hooks/useTaskLabels';` alongside the other hook imports
(near line 14, the `@/hooks` import).

## Allowed moves
- ONLY in `frontend/src/components/panels/AttemptHeaderActions.tsx`: replace the icon-only
  `ToggleGroup` block with the labeled `Tabs` block; add the badges cluster
  (`StatusBadge` + node `Badge` + outline `LabelBadge`s); add the `useTaskLabels` call; adjust imports
  to match (drop unused `ToggleGroup`/`ToggleGroupItem`/`Tooltip*`/icon/`useTranslation` imports, add
  `Tabs*`, `StatusBadge`, `Badge`, `LabelBadge`, `useTaskLabels`). Do not touch `ProjectTasks.tsx`,
  `new-card.tsx`, `TasksLayout.tsx`, `CompactLabelList.tsx`, or any other file. Do not add a standalone
  `text-lg font-semibold` title (it duplicates the breadcrumb).

## STOP triggers
- **Scope boundary (intentional, not a bug): this task does NOT add a standalone header title.** The
  task title already renders in the parent `NewCardHeader` `<Breadcrumb>` (ProjectTasks.tsx ~882–913);
  a `text-lg font-semibold` title here would duplicate it. If the executor believes a title MUST land
  here, STOP and reconcile — do not duplicate the breadcrumb title.
- **Placement deviation (intentional, surfaced — not a silent approximation):** SC18 describes a
  "badges row below the header." `NewCardHeader` (new-card.tsx:32–36) renders the `actions` slot as a
  **top-right inline flex row**, so the badges land inline in the actions cluster, not on a separate
  band below the breadcrumb. A literal below-header band requires editing `new-card.tsx` or
  `ProjectTasks.tsx` (both out of this task's `files:`). The badges cluster is grouped in its own
  flex container as the closest faithful rendering. If a literal below-header band is mandatory,
  STOP and reconcile (it needs a separate task that may edit `new-card.tsx`/`ProjectTasks.tsx`) —
  do NOT silently edit those files here.
- **Dropped modes — do NOT silently drop `preview`/`files`/`processes`.** The new 3-tab set covers
  only `diffs`/`terminal`/`null`. `preview` and `files` remain reachable via the keyboard cycle in
  ProjectTasks.tsx (~line 558: `const order: LayoutMode[] = [null, 'preview', 'diffs', 'files'];`).
  **`processes` is in neither the new tabs nor that cycle** — after this change it has no desktop
  affordance in this header. Before committing, EITHER confirm `processes` has another surface
  (re-grep `mode === 'processes'` across `frontend/src`) OR halt and record in the ledger that
  `processes` is now only programmatically reachable, for explicit sign-off. Do not delete the
  `processes` rendering branch in `ProjectTasks.tsx`.
- The `ToggleGroup` block body differs materially from the Before text (re-grep `value={mode ?? ''}`;
  if absent, halt — the file changed since decompose).
- `../ui/tabs` does not export `Tabs`/`TabsList`/`TabsTrigger` (halt; reconcile import).
- `@/components/common/StatusBadge` does not exist or lacks the `{ status }` prop (halt — task 014
  not applied; this task `depends_on: ["014"]`).
- `LabelBadge` does not accept `variant="outline"` (halt — task 009 not applied; this task
  `depends_on: ["009"]`. `LabelBadge`'s `variant?: 'solid' | 'outline'` prop is ADDED by task 009;
  on plain `main` it is absent and `tsc` will fail).
- `Badge` from `@/components/ui/badge` lacks a `secondary` variant (halt; reconcile — it is present
  today).

## Manual verification (record in decisions-ledger)
- `grep -n '<Tabs' frontend/src/components/panels/AttemptHeaderActions.tsx` → match.
- `grep -nE '<TabsTrigger value="(diff|logs|attempts)">' frontend/src/components/panels/AttemptHeaderActions.tsx`
  → the three literal tab labels (Diff / Logs / Attempts) present.
- `grep -n 'TODO(i18n): vk-swarm-node-ui-localize' frontend/src/components/panels/AttemptHeaderActions.tsx`
  → match (literal tab labels flagged for localization).
- `grep -c '<StatusBadge' frontend/src/components/panels/AttemptHeaderActions.tsx` → 2 (SC18:96 header
  dot + SC18:97 row outline badge).
- `grep -n 'variant="outline"' frontend/src/components/panels/AttemptHeaderActions.tsx` → label badges present.
- `grep -n 'source_node_name' frontend/src/components/panels/AttemptHeaderActions.tsx` → node badge present.
- `grep -n 'useTaskLabels' frontend/src/components/panels/AttemptHeaderActions.tsx` → labels hook wired.
- `grep -nE '\bt\(' frontend/src/components/panels/AttemptHeaderActions.tsx` → no match (and
  `grep -n 'useTranslation' ...` → no match: the now-unused `t`/`useTranslation` were removed).
- `grep -n 'ToggleGroup' frontend/src/components/panels/AttemptHeaderActions.tsx` → no match
  (old icon switcher removed).
- `cd frontend && npx tsc --noEmit` → passes.
- Browser: open a task detail attempt panel; confirm the Diff / Logs / Attempts labeled tabs switch
  the aux panel (Diff→diffs, Logs→terminal, Attempts→attempt history), and the status badge, node
  badge (when `source_node_name` present), and outline label badges render. Confirm no console errors.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 020` exits 0
