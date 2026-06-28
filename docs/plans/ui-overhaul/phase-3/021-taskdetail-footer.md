---
id: "021"
phase: 3
title: Task-detail footer — Merge filled-primary, Rebase sm, Open-in-IDE ghost (GitOperations)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/components/tasks/Toolbar/GitOperations.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC18]
---
## Failing test (write first)
N/A — covered by manual verification (greppable assertions + browser smoke-test below; the
change is presentational button-variant/size and a unit test cannot cheaply assert it).

## Change

### File: `frontend/src/components/tasks/Toolbar/GitOperations.tsx`

Per SC18 (ADR-0006) footer: **Merge** → filled primary, full-width (`variant="default" size="sm"
className="flex-1"`); **Rebase** → `size="sm"` (variant stays `outline`); add an **Open in IDE**
ghost-`sm` button to the actions row. Confirmed valid Button props:
`frontend/src/components/ui/button.tsx` defines variants `default`/`outline`/`ghost` and sizes
`default`/`xs`/`sm`.

**Open in IDE — wiring (READ `ActionsDropdown` + `useOpenInEditor`):** the Open-in-IDE action is the
`useOpenInEditor` hook (`@/hooks/useOpenInEditor`). `ActionsDropdown`
(`frontend/src/components/ui/actions-dropdown.tsx:40,76,168–172`) calls
`const openInEditor = useOpenInEditor(attempt?.id)` and invokes `openInEditor()` (icon `ExternalLink`,
label `t('actionsMenu.openInIde')`). `GitOperations` already receives `selectedAttempt: TaskAttempt`
(props, line 32–41), whose `.id` is always present — so this task wires the **same hook** as
`useOpenInEditor(selectedAttempt.id)` with **no prop threading required**. The label is a LITERAL
"Open in IDE" with a `// TODO(i18n): vk-swarm-node-ui-localize` comment (the existing
`actionsMenu.openInIde` key is in the `tasks` namespace; this task adds no `useTranslation` to keep
the edit surgical — `GitOperations` already imports `useTranslation`, but the new button uses a
literal so its localization is tracked under `vk-swarm-node-ui-localize` rather than coupling to the
dropdown's key).

**The PR button (between Merge and Rebase, ~lines 422–443) is intentionally NOT touched** — leave
its `variant="outline" size="xs"` and classes unchanged.

**Edit 1 — Merge button (~lines 401–420).** Switch from outline-`xs` to filled-`default`-`sm`,
full width. The existing `border-success text-success hover:bg-success` classes are **outline
styling** that contradicts a filled `variant="default"` — drop them and keep only the layout
classes plus the new `flex-1`:

- Before (lines ~401–420):
```tsx
            <Button
              onClick={handleMergeClick}
              disabled={
                mergeInfo.hasMergedPR ||
                mergeInfo.hasOpenPR ||
                merging ||
                hasConflictsCalculated ||
                isAttemptRunning ||
                ((branchStatus.commits_ahead ?? 0) === 0 &&
                  !pushSuccess &&
                  !mergeSuccess)
              }
              variant="outline"
              size="xs"
              className="border-success text-success hover:bg-success gap-1 shrink-0"
              aria-label={mergeButtonLabel}
            >
              <GitBranchIcon className="h-3.5 w-3.5" />
              <span className="truncate max-w-[10ch]">{mergeButtonLabel}</span>
            </Button>
```
- After:
```tsx
            <Button
              onClick={handleMergeClick}
              disabled={
                mergeInfo.hasMergedPR ||
                mergeInfo.hasOpenPR ||
                merging ||
                hasConflictsCalculated ||
                isAttemptRunning ||
                ((branchStatus.commits_ahead ?? 0) === 0 &&
                  !pushSuccess &&
                  !mergeSuccess)
              }
              variant="default"
              size="sm"
              className="flex-1 gap-1 shrink-0"
              aria-label={mergeButtonLabel}
            >
              <GitBranchIcon className="h-3.5 w-3.5" />
              <span className="truncate max-w-[10ch]">{mergeButtonLabel}</span>
            </Button>
```

**Edit 2 — Rebase button (~lines 445–462).** Bump size `xs` → `sm`; keep `variant="outline"` and
the existing classes:

- Before (lines ~445–462):
```tsx
            <Button
              onClick={handleRebaseDialogOpen}
              disabled={
                mergeInfo.hasMergedPR ||
                rebasing ||
                isAttemptRunning ||
                hasConflictsCalculated
              }
              variant="outline"
              size="xs"
              className="border-warning text-warning hover:bg-warning gap-1 shrink-0"
              aria-label={rebaseButtonLabel}
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${rebasing ? 'animate-spin' : ''}`}
              />
              <span className="truncate max-w-[10ch]">{rebaseButtonLabel}</span>
            </Button>
```
- After:
```tsx
            <Button
              onClick={handleRebaseDialogOpen}
              disabled={
                mergeInfo.hasMergedPR ||
                rebasing ||
                isAttemptRunning ||
                hasConflictsCalculated
              }
              variant="outline"
              size="sm"
              className="border-warning text-warning hover:bg-warning gap-1 shrink-0"
              aria-label={rebaseButtonLabel}
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${rebasing ? 'animate-spin' : ''}`}
              />
              <span className="truncate max-w-[10ch]">{rebaseButtonLabel}</span>
            </Button>
```

**Edit 3 — Add the Open-in-IDE ghost-`sm` button to the actions row.**

First, wire the hook. Add the import alongside the existing hook import (`useGitOperations`, line 30):

```tsx
import { useOpenInEditor } from '@/hooks/useOpenInEditor';
```
Add the icon to the existing lucide import block (lines 1–10) — `ExternalLink` is the icon
`ActionsDropdown` uses for this action (NOTE: `ExternalLink` is **already imported** in this file at
line 9 for the open-PR chip, so do NOT add a duplicate import; reuse it). Then create the hook
instance near the other hook calls (after `const git = useGitOperations(...)`, ~line 57):

```tsx
  const openInEditor = useOpenInEditor(selectedAttempt.id);
```

Insert the button into the actions cluster (the `branchStatus && (<div className={actionsClasses}>…`
block, ~lines 399–464). Place it **after the Rebase button**, as the last child of `actionsClasses`:

- Before (the Rebase button's closing `</Button>` then the actions `</div>`, ~lines 462–464):
```tsx
              <span className="truncate max-w-[10ch]">{rebaseButtonLabel}</span>
            </Button>
          </div>
        )}
```
- After (add the ghost Open-in-IDE button before the actions-cluster `</div>`):
```tsx
              <span className="truncate max-w-[10ch]">{rebaseButtonLabel}</span>
            </Button>

            {/* TODO(i18n): vk-swarm-node-ui-localize — literal "Open in IDE". */}
            <Button
              onClick={() => openInEditor()}
              variant="ghost"
              size="sm"
              className="gap-1 shrink-0"
              aria-label="Open in IDE"
            >
              <ExternalLink className="h-3.5 w-3.5" />
              <span className="truncate max-w-[10ch]">Open in IDE</span>
            </Button>
          </div>
        )}
```
(`useOpenInEditor` already no-ops when its `attemptId` is falsy and surfaces the editor-selection
dialog on failure, so no extra guard/disabled logic is needed here.)

## Allowed moves
- ONLY: the Merge button (`variant`/`size`/`className`), the Rebase button (`size`), and the NEW
  Open-in-IDE ghost-`sm` button (plus its `useOpenInEditor` import + hook call) in
  `frontend/src/components/tasks/Toolbar/GitOperations.tsx`. Do not touch the PR button (between Merge
  and Rebase), the branch chips, status chips, the change-target `Settings` button, or any other
  handler/state. Do not thread new props (the hook needs only `selectedAttempt.id`, already in props).

## STOP triggers
- The Merge button body differs materially from the Before text (re-grep
  `border-success text-success hover:bg-success`; if absent, halt — the file changed since decompose).
- `default`/`sm`/`ghost` is NOT a valid Button variant/size (re-grep `frontend/src/components/ui/button.tsx`;
  all present today — halt only if the component changed).
- `useOpenInEditor` does not exist at `@/hooks/useOpenInEditor` or its signature is no longer
  `useOpenInEditor(attemptId?: string, …)` returning a callable (re-grep the hook; it is the same one
  `ActionsDropdown` uses today — halt and reconcile the wiring if it changed).
- `selectedAttempt` is no longer a `GitOperations` prop / lacks `.id` (re-grep the `GitOperationsProps`
  interface, lines ~32–41 — halt; the Open-in-IDE button would have no attempt to open).
- `ExternalLink` is no longer imported in this file (re-grep line ~9 — if absent, add it to the lucide
  import block rather than assuming it is present).

## Manual verification (record in decisions-ledger)
- `grep -n 'variant="default"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → Merge
  button matches (filled).
- `grep -n 'flex-1' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → Merge button matches.
- `grep -c 'size="sm"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → ≥3 (Merge + Rebase + Open-in-IDE).
- `grep -n 'variant="ghost"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → the Open-in-IDE
  button matches (the change-target `Settings` button is `size="xs"`; the ghost-`sm` one is the new IDE button).
- `grep -n 'useOpenInEditor' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → hook imported + called.
- `grep -n 'aria-label="Open in IDE"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → match.
- `cd frontend && npx tsc --noEmit` → passes.
- Browser: open a task detail panel with branch status; Merge renders filled-primary full-width,
  Rebase renders outline at `sm`, and an "Open in IDE" ghost button renders in the actions row and
  opens the attempt in the configured editor on click. No console errors.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 021` exits 0
