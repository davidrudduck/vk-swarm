---
id: "013"
phase: 3
title: Kanban grid minmax(264px,1fr) + empty-column ANSI empty-state block
status: passed
depends_on: ["010", "003"]
parallel: false
conflicts_with: ["010"]
files:
  - frontend/src/components/ui/shadcn-io/kanban/index.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC9, SC10]
---
## Failing test (write first)
N/A — covered by manual verification (greppable assertions below). Builds on task 010, which edits
the same file; this task is chained via `depends_on` to avoid a mid-stream conflict.

## Change

This task has **three** sub-edits: a value import, the grid min-width, and the empty-state block.

### File: `frontend/src/components/ui/shadcn-io/kanban/index.tsx`

**Anchor 1 — add the `Children` value import (~line 21).** The empty-state (Anchor 3) counts the
column's children with `Children.count(...)`. Today line 21 imports only *types* from `'react'`
(JSX uses the new transform, so `React` is NOT in scope). Add the named `Children` value import.

- Before:
```tsx
import { type ReactNode, type Ref, type KeyboardEvent } from 'react';
```
- After:
```tsx
import { Children, type ReactNode, type Ref, type KeyboardEvent } from 'react';
```

**Anchor 2 — column grid min-width (inside `KanbanProvider`, ~line 343).** Widen the column track
floor and let columns flex to fill (SC9).

- Before:
```tsx
          'inline-grid grid-flow-col auto-cols-[minmax(200px,400px)] divide-x border-x items-stretch min-h-full',
```
- After:
```tsx
          'inline-grid grid-flow-col auto-cols-[minmax(264px,1fr)] divide-x border-x items-stretch min-h-full',
```

**Anchor 3 — empty-column empty-state (inside `KanbanCards`, ~lines 144–146) (SC10).** `KanbanCards`
is the column body; both consumers (`TaskKanbanBoard.tsx`, `AllProjectsTasks.tsx`) pass
`items.map(...)` as `children`, so an empty column yields zero children. Render an ANSI empty-state
when `Children.count(children) === 0`. The `vks-ansi-dither` / `vks-scanlines` classes are added by
task 003 (dependency).

- Before:
```tsx
export const KanbanCards = ({ children, className }: KanbanCardsProps) => (
  <div className={cn('flex flex-1 flex-col', className)}>{children}</div>
);
```
- After:
```tsx
export const KanbanCards = ({ children, className }: KanbanCardsProps) => (
  <div className={cn('flex flex-1 flex-col', className)}>
    {Children.count(children) === 0 ? (
      <div className="vks-ansi-dither vks-scanlines rounded-md border min-h-[80px] flex items-center justify-center">
        <span className="font-mono text-xs text-muted-foreground">
          ░▒ no tasks ▒░
        </span>
      </div>
    ) : (
      children
    )}
  </div>
);
```

## Allowed moves
- ONLY the three edits above, all inside `frontend/src/components/ui/shadcn-io/kanban/index.tsx`.
- Do NOT alter the card-state classes touched by task 010 (ring, add-button, status dot, count badge).

## STOP triggers
- The grid line differs materially from Anchor 2's Before text (re-grep `minmax(200px,400px)`; if
  absent, halt — the file changed since decompose).
- The `KanbanCards` body differs materially from Anchor 3's Before text, OR `KanbanCards` no longer
  renders `{children}` into a single `<div>` (halt + report — cannot locate the empty-render path).
- `Children` is already imported from `'react'` on line 21 (halt; reconcile rather than duplicate).

## Manual verification (record in decisions-ledger)
- `grep 'minmax(264px' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match (SC9).
- `grep 'no tasks' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match (SC10).
- `grep -- 'import { Children,' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- `grep 'vks-ansi-dither vks-scanlines' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 013` exits 0
