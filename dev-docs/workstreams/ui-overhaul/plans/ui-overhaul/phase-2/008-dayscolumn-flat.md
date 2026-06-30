---
id: "008"
phase: 2
title: "DaysInColumnBadge flat variant + literal {n}d (no 7d+ cap)"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/components/tasks/DaysInColumnBadge.tsx
  - frontend/src/utils/daysInColumn.ts
  - frontend/src/utils/__tests__/daysInColumn.test.ts
  - frontend/src/components/tasks/__tests__/DaysInColumnBadge.test.tsx
irreversible: false
scope_test: "frontend/src/utils/__tests__"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
The existing `frontend/src/utils/__tests__/daysInColumn.test.ts` ENCODES the old behaviour and so
must be updated in lockstep (it is in `files`). It currently asserts `7d+` for 7/14/100 days and
imports + exercises `getDaysStyle` (a `describe('getDaysStyle', …)` block, 7 cases). After this task
those assertions are wrong and `getDaysStyle` no longer exists. The updated test (below) becomes the
failing-then-passing spec: literal `7d`/`14d`/`100d`, and the `getDaysStyle` block + import removed.

## Change

Two corrections:
(a) `formatDaysInColumn` caps at `7d+` for ≥7 days — remove the cap so it returns the literal
    `${days}d` for any day count ≥1.
(b) `DaysInColumnBadge` uses age-graduated colour styling (`getDaysStyle`: neutral → amber → red).
    Flatten it to a single `secondary` badge style; drop the graduation.

`getDaysStyle` has exactly two consumers — `DaysInColumnBadge.tsx` and the test file (verified at
decompose time: `grep -rn 'getDaysStyle' frontend/src` → only `daysInColumn.ts` definition,
`DaysInColumnBadge.tsx` import/call, and `daysInColumn.test.ts`). Both are in `files`, so removing
the function is safe and self-contained.

### File: `frontend/src/utils/daysInColumn.ts`

**Anchor 1 — `formatDaysInColumn()` (~lines 33–37).** Remove the `7d+` cap.

- Before:
```ts
export function formatDaysInColumn(days: number): string | null {
  if (days < 1) return null;
  if (days >= 7) return '7d+';
  return `${days}d`;
}
```
- After:
```ts
export function formatDaysInColumn(days: number): string | null {
  if (days < 1) return null;
  return `${days}d`;
}
```

Also update the doc comment immediately above (~lines 28–32) so it no longer claims a `7d+` cap.

- Before:
```ts
/**
 * Format days into a display string.
 * Returns null if days is 0 (< 1 day old).
 * Returns "7d+" for 7 or more days.
 */
```
- After:
```ts
/**
 * Format days into a display string.
 * Returns null if days is 0 (< 1 day old).
 * Returns the literal "{n}d" for any day count >= 1 (no upper cap).
 */
```

**Anchor 2 — `getDaysStyle()` (~lines 39–55).** After the badge is flattened (Anchor 4) this
function and its export become unused. Remove the entire `getDaysStyle` function INCLUDING its doc
comment (~lines 39–55). Record the removal in the decisions-ledger.

- Before:
```ts
/**
 * Get Tailwind classes for styling the days badge based on age.
 * - 1-2 days: neutral/subtle styling
 * - 3-6 days: warning (amber) styling
 * - 7+ days: strong warning (red) styling
 */
export function getDaysStyle(days: number): string {
  if (days < 1) return '';
  if (days <= 2) {
    return 'bg-muted text-muted-foreground';
  }
  if (days <= 6) {
    return 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400';
  }
  // 7+ days
  return 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400';
}
```
- After: (function deleted entirely)

### File: `frontend/src/utils/__tests__/daysInColumn.test.ts`

**Anchor 3 — import + `getDaysStyle` cases + `7d+` expectations.** Drop the `getDaysStyle` import,
fix the three capped expectations to literals, and delete the whole `getDaysStyle` describe block.

- Before (import, lines 2–6):
```ts
import {
  getDaysInColumn,
  formatDaysInColumn,
  getDaysStyle,
} from '../daysInColumn';
```
- After:
```ts
import { getDaysInColumn, formatDaysInColumn } from '../daysInColumn';
```

- Before (the three capped `formatDaysInColumn` cases, lines 87–97):
```ts
    it('returns "7d+" for 7 days', () => {
      expect(formatDaysInColumn(7)).toBe('7d+');
    });

    it('returns "7d+" for 14 days', () => {
      expect(formatDaysInColumn(14)).toBe('7d+');
    });

    it('returns "7d+" for 100 days', () => {
      expect(formatDaysInColumn(100)).toBe('7d+');
    });
```
- After:
```ts
    it('returns "7d" for 7 days', () => {
      expect(formatDaysInColumn(7)).toBe('7d');
    });

    it('returns "14d" for 14 days', () => {
      expect(formatDaysInColumn(14)).toBe('14d');
    });

    it('returns "100d" for 100 days', () => {
      expect(formatDaysInColumn(100)).toBe('100d');
    });
```

- Before (the entire `getDaysStyle` describe block, lines 100–140, sitting just before the two
  closing `});` of the file):
```ts
  describe('getDaysStyle', () => {
    it('returns empty string for 0 days', () => {
      expect(getDaysStyle(0)).toBe('');
    });

    it('returns neutral style for 1 day', () => {
      const style = getDaysStyle(1);
      expect(style).toContain('bg-muted');
      expect(style).toContain('text-muted-foreground');
    });

    it('returns neutral style for 2 days', () => {
      const style = getDaysStyle(2);
      expect(style).toContain('bg-muted');
      expect(style).toContain('text-muted-foreground');
    });

    it('returns warning (amber) style for 3 days', () => {
      const style = getDaysStyle(3);
      expect(style).toContain('bg-amber');
      expect(style).toContain('text-amber');
    });

    it('returns warning (amber) style for 6 days', () => {
      const style = getDaysStyle(6);
      expect(style).toContain('bg-amber');
      expect(style).toContain('text-amber');
    });

    it('returns strong warning (red) style for 7 days', () => {
      const style = getDaysStyle(7);
      expect(style).toContain('bg-red');
      expect(style).toContain('text-red');
    });

    it('returns strong warning (red) style for 14 days', () => {
      const style = getDaysStyle(14);
      expect(style).toContain('bg-red');
      expect(style).toContain('text-red');
    });
  });
```
- After: (describe block deleted entirely; the outer `describe('daysInColumn utilities', …)` still
  closes with its `});`)

### File: `frontend/src/components/tasks/DaysInColumnBadge.tsx`

**Anchor 4 — import (lines 2–6).** Drop `getDaysStyle` from the import (it is removed in Anchor 2).

- Before:
```tsx
import {
  getDaysInColumn,
  formatDaysInColumn,
  getDaysStyle,
} from '@/utils/daysInColumn';
```
- After:
```tsx
import { getDaysInColumn, formatDaysInColumn } from '@/utils/daysInColumn';
```

**Anchor 5 — component body (~lines 15–46).** Remove the `getDaysStyle` call and switch the badge
to a flat `secondary` style. Update the doc comment (~lines 15–22) so it no longer describes
age-graduated styling.

- Before:
```tsx
/**
 * Badge component showing how many days a task has been in its current column.
 * Returns null if less than 1 day old.
 * Shows age-appropriate styling:
 * - 1-2 days: neutral/subtle
 * - 3-6 days: amber warning
 * - 7+ days: red strong warning
 */
export function DaysInColumnBadge({
  activityAt,
  className,
}: DaysInColumnBadgeProps) {
  const days = getDaysInColumn(activityAt);
  const formatted = formatDaysInColumn(days);

  // Don't render if less than 1 day
  if (!formatted) return null;

  const styleClasses = getDaysStyle(days);

  return (
    <span
      className={cn(
        'inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium',
        styleClasses,
        className
      )}
      title={`${days} day${days === 1 ? '' : 's'} in this column`}
    >
      {formatted}
    </span>
  );
}
```
- After:
```tsx
/**
 * Badge component showing how many days a task has been in its current column.
 * Returns null if less than 1 day old. Renders a flat, neutral `secondary` badge
 * regardless of age (no age-graduated colours).
 */
export function DaysInColumnBadge({
  activityAt,
  className,
}: DaysInColumnBadgeProps) {
  const days = getDaysInColumn(activityAt);
  const formatted = formatDaysInColumn(days);

  // Don't render if less than 1 day
  if (!formatted) return null;

  return (
    <span
      className={cn(
        'inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium',
        'bg-secondary text-secondary-foreground',
        className
      )}
      title={`${days} day${days === 1 ? '' : 's'} in this column`}
    >
      {formatted}
    </span>
  );
}
```

## Allowed moves
- ONLY: remove the `7d+` cap in `formatDaysInColumn`; delete `getDaysStyle` and its export; update
  the test file (drop `getDaysStyle` import + describe block, fix the three `7d+` expectations to
  `7d`/`14d`/`100d`); drop the `getDaysStyle` import in the badge; flatten the badge to
  `bg-secondary text-secondary-foreground`; update the two doc comments. Do not touch
  `getDaysInColumn`, the props interface, the other test cases, or `cn`/`className` plumbing.

## STOP triggers
- `formatDaysInColumn` no longer contains `if (days >= 7) return '7d+';` (halt — already changed).
- `getDaysStyle` is imported by a file OTHER than `DaysInColumnBadge.tsx` or `daysInColumn.test.ts`
  (`grep -rn 'getDaysStyle' frontend/src` returns a fourth location): halt — a consumer outside this
  task's `files`; do NOT delete, reconcile first.
- The `DaysInColumnBadge` body differs materially from the Before text (the `getDaysStyle(days)`
  call is absent): halt.

## Manual verification (record in decisions-ledger)
- `grep -rn "7d+" frontend/src` → no matches (cap removed from source AND test).
- `grep -rn 'getDaysStyle' frontend/src` → no matches (fully removed from source, badge, and test).
- `grep -n 'bg-secondary text-secondary-foreground' frontend/src/components/tasks/DaysInColumnBadge.tsx`
  → match (flat style).
- `cd frontend && npx tsc --noEmit` → passes (no unused/missing-export errors).
- `cd frontend && npx vitest run src/utils/__tests__/daysInColumn.test.ts` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 008` exits 0 (scope_test is now a directory; gate auto-detects vitest)
