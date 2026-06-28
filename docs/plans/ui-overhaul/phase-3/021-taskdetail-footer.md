---
id: "021"
phase: 3
title: Task-detail footer — Merge filled-primary, Rebase sm (GitOperations)
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
className="flex-1"`); **Rebase** → `size="sm"` (variant stays `outline`). Confirmed valid Button
props: `frontend/src/components/ui/button.tsx` defines variants `default`/`outline`/`ghost` and
sizes `default`/`xs`/`sm`.

**Open in IDE:** there is **no** Open-in-IDE button in this component (confirmed by grep —
`GitOperations.tsx` renders only Merge / PR / Rebase in the actions cluster). The SC18 "Open in IDE
→ ghost sm" clause is therefore **out of scope for this task** — that affordance lives elsewhere
(e.g. `ActionsDropdown`); do not fabricate it here. Recorded in `## STOP triggers`.

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

## Allowed moves
- ONLY the Merge button (`variant`/`size`/`className`) and the Rebase button (`size`) in
  `frontend/src/components/tasks/Toolbar/GitOperations.tsx`. Do not touch the PR button (between
  them), the branch chips, status chips, the change-target `Settings` button, or any handler/state.
  Do not add an Open-in-IDE button.

## STOP triggers
- The Merge button body differs materially from the Before text (re-grep
  `border-success text-success hover:bg-success`; if absent, halt — the file changed since decompose).
- `default` or `sm` is NOT a valid Button variant/size (re-grep `frontend/src/components/ui/button.tsx`;
  both are present today — halt only if the component changed).
- An Open-in-IDE button DOES exist in this footer after all (re-grep `ide`/`openInIde` in this file
  is currently zero matches) — if one appears, set it `variant="ghost" size="sm"` and document;
  otherwise leave the SC18 Open-in-IDE clause to its owning surface (`ActionsDropdown`) and note in
  the ledger.

## Manual verification (record in decisions-ledger)
- `grep -n 'variant="default"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → Merge
  button matches (filled).
- `grep -n 'flex-1' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → Merge button matches.
- `grep -c 'size="sm"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → ≥2 (Merge + Rebase).
- `cd frontend && npx tsc --noEmit` → passes.
- Browser: open a task detail panel with branch status; Merge renders filled-primary full-width,
  Rebase renders outline at `sm`. No console errors.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 021` exits 0
