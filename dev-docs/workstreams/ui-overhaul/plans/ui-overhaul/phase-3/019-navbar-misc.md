---
id: "019"
phase: 3
title: "ProjectSwitcher folder icon + SearchBar fixed width"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/components/layout/ProjectSwitcher.tsx
  - frontend/src/components/SearchBar.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
N/A — two small presentational edits (an icon glyph + a width class); covered by manual verification
(greppable assertions below). No cheap unit test adds coverage beyond `tsc --noEmit` + grep.
`WAI_TEST_CMD="true"`.

## Change

Two independent, single-file edits. They share no file with each other, so order is irrelevant.

### File 1: `frontend/src/components/layout/ProjectSwitcher.tsx`

Add a `<FolderOpen>` icon (lucide-react) inside the trigger button, before the
`<span className="truncate">{displayValue}</span>`. `FolderOpen` is NOT currently imported in this
file (the lucide import on line 3 is `{ Check, ChevronsUpDown }`), so it must be added.

**Anchor 1 — the lucide-react import (line 3).**
- Before:
```tsx
import { Check, ChevronsUpDown } from 'lucide-react';
```
- After:
```tsx
import { Check, ChevronsUpDown, FolderOpen } from 'lucide-react';
```

**Anchor 2 — the trigger button content (lines 104-116).**
- Before:
```tsx
        <Button
          variant="ghost"
          role="combobox"
          aria-expanded={open}
          disabled={isLoading}
          className={cn(
            'w-auto max-w-[200px] h-8 justify-between text-sm px-2',
            className
          )}
        >
          <span className="truncate">{displayValue}</span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
```
- After:
```tsx
        <Button
          variant="ghost"
          role="combobox"
          aria-expanded={open}
          disabled={isLoading}
          className={cn(
            'w-auto max-w-[200px] h-8 justify-between text-sm px-2',
            className
          )}
        >
          <FolderOpen className="mr-2 h-4 w-4 shrink-0 opacity-70" />
          <span className="truncate">{displayValue}</span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
```

### File 2: `frontend/src/components/SearchBar.tsx`

Replace the responsive width (`w-64 sm:w-72`) on the wrapper `<div>` with a fixed `w-[260px]`.

**Anchor 3 — the wrapper `<div>` (line 23).**
- Before:
```tsx
      <div className={cn('relative w-64 sm:w-72', className)}>
```
- After:
```tsx
      <div className={cn('relative w-[260px]', className)}>
```

## Allowed moves
- ONLY: in `ProjectSwitcher.tsx`, add `FolderOpen` to the lucide-react import (Anchor 1) and render
  `<FolderOpen … />` before the `displayValue` span in the trigger (Anchor 2); in `SearchBar.tsx`,
  swap `w-64 sm:w-72` → `w-[260px]` on the wrapper div (Anchor 3). No other change to either file, no
  other file.

## STOP triggers
- `FolderOpen` is already imported in `ProjectSwitcher.tsx` (halt — reconcile; do not duplicate the
  import).
- The trigger button content differs materially from the Before text (re-grep
  `<span className="truncate">{displayValue}</span>` to locate; if absent, halt).
- `SearchBar.tsx` line 23 does not contain `w-64 sm:w-72` (halt — width already changed since
  decompose; reconcile).

## Manual verification (record in decisions-ledger)
- `grep 'FolderOpen' frontend/src/components/layout/ProjectSwitcher.tsx` → ≥2 matches (import + JSX).
- `grep 'w-\[260px\]' frontend/src/components/SearchBar.tsx` → match.
- `grep -c 'w-64 sm:w-72' frontend/src/components/SearchBar.tsx` → `0` (old width gone).
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 019` exits 0
