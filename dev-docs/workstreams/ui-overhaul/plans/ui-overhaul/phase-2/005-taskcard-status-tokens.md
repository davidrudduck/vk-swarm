---
id: "005"
phase: 2
title: TaskCard + AllProjectsTaskCard status strips → --status-* tokens
status: passed
depends_on: ["002"]
parallel: false
conflicts_with: ["006"]
files:
  - frontend/src/components/tasks/TaskCard.tsx
  - frontend/src/components/tasks/AllProjectsTaskCard.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC5a, SC5b]
---
## Failing test (write first)
N/A — covered by manual verification (greppable assertions below + `tsc --noEmit`). No cheap unit
test asserts the rendered strip colour; the SC5a/SC5b greps are the gate.

## Change

This is a **SIBLING-ALIGNED** task: `TaskCard.tsx` and `AllProjectsTaskCard.tsx` carry an IDENTICAL
`statusStripColors` map. Per the rubric, the task MUST:

1. Read `AllProjectsTaskCard.tsx`; confirm its `statusStripColors` map matches `TaskCard.tsx`'s.
2. Apply the identical token mapping to both.
3. Justify any divergence in the decisions ledger.

Tokens from task 002 are bare HSL channel triplets (e.g. `--status-done: 152 100% 50%`), so they
MUST be consumed as `hsl(var(--status-*))` inside the Tailwind arbitrary value. A bare
`bg-[var(--status-done)]` would emit `background-color: 152 100% 50%`, which is invalid and renders
no colour. The `hsl()` wrapper still satisfies SC5b's `var(--status-` grep. (This reconciles with the
shorthand `before:bg-[var(--status-done)]` quoted in `plan.md`'s contracts section — record the
reconciliation in the ledger.)

### File: `frontend/src/components/tasks/TaskCard.tsx`

**Anchor — `statusStripColors` map (~lines 27–33).**

- Before:
```typescript
const statusStripColors: Record<TaskStatus, string> = {
  todo: 'before:bg-neutral-400 dark:before:bg-neutral-500',
  inprogress: 'before:bg-blue-500',
  inreview: 'before:bg-amber-500',
  done: 'before:bg-green-500',
  cancelled: 'before:bg-red-500',
};
```
- After:
```typescript
const statusStripColors: Record<TaskStatus, string> = {
  todo: 'before:bg-[hsl(var(--status-todo))]',
  inprogress: 'before:bg-[hsl(var(--status-inprogress))]',
  inreview: 'before:bg-[hsl(var(--status-inreview))]',
  done: 'before:bg-[hsl(var(--status-done))]',
  cancelled: 'before:bg-[hsl(var(--status-cancelled))]',
};
```

### File: `frontend/src/components/tasks/AllProjectsTaskCard.tsx`

**Anchor — `statusStripColors` map (~lines 12–18).** The map text is IDENTICAL to TaskCard's Before
above (verified). Apply the IDENTICAL After.

- Before:
```typescript
const statusStripColors: Record<TaskStatus, string> = {
  todo: 'before:bg-neutral-400 dark:before:bg-neutral-500',
  inprogress: 'before:bg-blue-500',
  inreview: 'before:bg-amber-500',
  done: 'before:bg-green-500',
  cancelled: 'before:bg-red-500',
};
```
- After:
```typescript
const statusStripColors: Record<TaskStatus, string> = {
  todo: 'before:bg-[hsl(var(--status-todo))]',
  inprogress: 'before:bg-[hsl(var(--status-inprogress))]',
  inreview: 'before:bg-[hsl(var(--status-inreview))]',
  done: 'before:bg-[hsl(var(--status-done))]',
  cancelled: 'before:bg-[hsl(var(--status-cancelled))]',
};
```

## Allowed moves
- ONLY replace the `statusStripColors` map body in BOTH files with the token-based map above.
- Do NOT touch the strip-width classes. Verified out-of-scope divergence: `TaskCard.tsx` uses
  `before:w-[4px]` and `AllProjectsTaskCard.tsx` uses `before:w-[3px]`; this lives on a line neither
  task touches. The `statusStripColors` MAPS are identical, so the sibling-alignment "justify any
  divergence" clause has nothing to justify for THIS edit — record in the ledger: "maps identical;
  applied identically; width difference noted, out of scope."
- No other class, no other component, no public-token-layer change.

## STOP triggers
- The `statusStripColors` map text in EITHER file differs from the Before block above (halt — file
  changed since decompose; re-locate by grepping `before:bg-green-500`).
- Either file lacks a `statusStripColors` map (halt).

## Manual verification (record in decisions-ledger)
- `grep -rE 'bg-green-500|bg-red-500|bg-amber-500|bg-blue-500' frontend/src/components/tasks/TaskCard.tsx frontend/src/components/tasks/AllProjectsTaskCard.tsx` → zero matches in the two files (SC5a; note: the whole-directory + `TaskCountPills.tsx` form of this grep is the Phase-4 gate — `TaskCountPills` is handled by task 007, so scope THIS grep to the two files).
- `grep 'var(--status-' frontend/src/components/tasks/TaskCard.tsx frontend/src/components/tasks/AllProjectsTaskCard.tsx` → ≥1 match in EACH file (SC5b).
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 005` exits 0
