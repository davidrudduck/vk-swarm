---
id: "010"
phase: 2
title: Kanban card-state fixes (selected ring, add-button size, status dot, count badge bg)
status: passed
depends_on: ["002"]
parallel: false
conflicts_with: ["013"]
files:
  - frontend/src/components/ui/shadcn-io/kanban/index.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
N/A — covered by manual verification (visual class changes; greppable assertions below).

## Change

### File: `frontend/src/components/ui/shadcn-io/kanban/index.tsx`

**Anchor 1 — selected-card ring (inside `KanbanCard`, ~line 118).** The open/selected card uses a
muted inset secondary ring; switch it to the brand primary ring.

- Before:
```tsx
        isOpen && 'ring-2 ring-secondary-foreground ring-inset',
```
- After:
```tsx
        isOpen && 'ring-2 ring-primary',
```

**Anchor 2 — column add-task button (inside `KanbanHeader`, ~line 244).** This is the ghost icon
`<Button>` wrapping the `<Plus />` glyph. Its `h-0` collapses the button to zero height, hiding the
hit-target (A-class bug). Give it a visible 6×6 icon-button footprint. Change ONLY the size classes
(`h-0` → `h-6 w-6`); leave the colour classes untouched.

- Before:
```tsx
              className="m-0 p-0 h-0 text-foreground/50 hover:text-foreground"
```
- After:
```tsx
              className="m-0 p-0 h-6 w-6 text-foreground/50 hover:text-foreground"
```

**Anchor 3 — column status dot (inside `KanbanHeader`, ~line 220).** Nudge the dot from 10px to 9px.

- Before:
```tsx
          className="h-2.5 w-2.5 rounded-full shrink-0"
```
- After:
```tsx
          className="h-[9px] w-[9px] rounded-full shrink-0"
```

**Anchor 4 — column count badge background (inside `KanbanHeader`, ~line 228).** Swap the generic
`bg-muted` for the `--surface-card` token (added in task 002). The token is a bare HSL triplet, so
it MUST be wrapped in `hsl(...)`. Replace ONLY `bg-muted` with `bg-[hsl(var(--surface-card))]`;
leave every other class in place.

- Before:
```tsx
            className="ml-0.5 px-1.5 py-0.5 rounded text-xs bg-muted text-muted-foreground font-normal tabular-nums"
```
- After:
```tsx
            className="ml-0.5 px-1.5 py-0.5 rounded text-xs bg-[hsl(var(--surface-card))] text-muted-foreground font-normal tabular-nums"
```

## Allowed moves
- ONLY the four class-string edits above, all inside `frontend/src/components/ui/shadcn-io/kanban/index.tsx`.
- Do NOT touch the grid line (`auto-cols-[minmax(...)]`), `KanbanCards`, imports, or any other file
  (that grid/empty-state work belongs to task 013, which depends on this task).

## STOP triggers
- Any of the four Before strings differs materially from the file (re-grep the literal class string to
  locate; if absent, halt — the file changed since decompose).
- The add-button `<Button>` is no longer an icon button wrapping `<Plus />` (halt; reconcile).

## Manual verification (record in decisions-ledger)
- `grep -- "isOpen && 'ring-2 ring-primary'" frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- `grep -- 'm-0 p-0 h-6 w-6 text-foreground/50' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- `grep -- 'h-\[9px\] w-\[9px\] rounded-full shrink-0' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- `grep -- 'bg-\[hsl(var(--surface-card))\]' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- `grep -- 'bg-muted text-muted-foreground font-normal' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → NO match (old class gone).
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 010` exits 0
