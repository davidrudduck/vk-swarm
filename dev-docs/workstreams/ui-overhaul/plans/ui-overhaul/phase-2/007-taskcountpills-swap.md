---
id: "007"
phase: 2
title: TaskCountPills inprogress/inreview colour swap
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/components/projects/TaskCountPills.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC8]
---
## Failing test (write first)
N/A — covered by manual verification (visual colour assignment; no cheap unit test). Verified
instead by reading the corrected source and running the SC5a grep below.

## Change

A-class bug: the `inprogress` and `inreview` pills carry SWAPPED colours. `inprogress` is rendered
amber and `inreview` is rendered blue. The design language assigns BLUE to in-progress and AMBER to
in-review (matching the status-token semantics: `--status-inprogress` is blue `217 91% 60%`,
`--status-inreview` is amber `43 100% 50%`). Swap the two `colorClass` + `bgClass` pairs.

This file is also in scope of the Phase 4 SC5a removal grep, which strips the bare status fill
tokens `bg-green-500` / `bg-red-500` / `bg-amber-500` / `bg-blue-500`. After this swap the file
still uses Tailwind blue/amber utilities, but only in `text-*` and tinted `bg-amber-50` / `bg-blue-50`
forms — NOT the bare `bg-amber-500` / `bg-blue-500` tokens — so SC5a is unaffected. Verified at
decompose time: `grep -nE 'bg-amber-500|bg-blue-500|bg-green-500|bg-red-500'
frontend/src/components/projects/TaskCountPills.tsx` → zero matches (exit 1). The Manual
verification section re-asserts this.

### File: `frontend/src/components/projects/TaskCountPills.tsx`

**Anchor 1 — `inprogress` entry (the `colorClass`/`bgClass` lines, ~lines 44–46).** Change from
amber to blue.

- Before:
```tsx
      colorClass: 'text-amber-600 dark:text-amber-400',
      bgClass:
        'bg-amber-50/50 hover:bg-amber-50 dark:bg-amber-900/20 dark:hover:bg-amber-900/30',
```
- After:
```tsx
      colorClass: 'text-blue-600 dark:text-blue-400',
      bgClass:
        'bg-blue-50/50 hover:bg-blue-50 dark:bg-blue-900/20 dark:hover:bg-blue-900/30',
```

**Anchor 2 — `inreview` entry (the `colorClass`/`bgClass` lines, ~lines 53–55).** Change from blue
to amber.

- Before:
```tsx
      colorClass: 'text-blue-600 dark:text-blue-400',
      bgClass:
        'bg-blue-50/50 hover:bg-blue-50 dark:bg-blue-900/20 dark:hover:bg-blue-900/30',
```
- After:
```tsx
      colorClass: 'text-amber-600 dark:text-amber-400',
      bgClass:
        'bg-amber-50/50 hover:bg-amber-50 dark:bg-amber-900/20 dark:hover:bg-amber-900/30',
```

Leave the `todo` and `done` entries UNCHANGED.

## Allowed moves
- ONLY swap the `colorClass` + `bgClass` value pairs between the `inprogress` and `inreview`
  entries. Do not touch `key`, `label`, `compactLabel`, `count`, the `todo`/`done` entries, the
  JSX, or any other file.

## STOP triggers
- The `inprogress` entry is no longer amber, or the `inreview` entry is no longer blue (halt — the
  file changed since decompose; the swap may already be applied or the anchors moved).
- The `colorClass`/`bgClass` literals differ materially from the Before text (re-grep
  `text-amber-600 dark:text-amber-400` and `text-blue-600 dark:text-blue-400` to locate; if either
  is absent, halt).

## Manual verification (record in decisions-ledger)
- Read the corrected source: `inprogress` entry uses `text-blue-600 dark:text-blue-400` +
  `bg-blue-50/50 …`; `inreview` entry uses `text-amber-600 dark:text-amber-400` + `bg-amber-50/50 …`.
- `grep -nE 'bg-amber-500|bg-blue-500|bg-green-500|bg-red-500'
  frontend/src/components/projects/TaskCountPills.tsx` → no matches (SC5a unaffected).
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 007` exits 0
