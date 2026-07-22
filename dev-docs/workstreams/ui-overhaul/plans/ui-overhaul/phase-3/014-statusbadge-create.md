---
id: "014"
phase: 3
title: Create StatusBadge component (8px status dot + optional label)
status: passed
depends_on: ["002"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/common/StatusBadge.tsx
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: []
---
## Failing test (write first)
N/A — covered by manual verification (scope_test N/A; greppable assertions below). A vitest is not
cheap here and adds no coverage a tsc + grep gate doesn't already give. `WAI_TEST_CMD="true"`.

## Change

NEW component per the plan's Shared-interface contract: **named export `StatusBadge`** from
`frontend/src/components/common/StatusBadge.tsx`, props
`{ status: TaskStatus; showLabel?: boolean; className?: string }`. Renders an 8px dot coloured by
the `--status-*` tokens (added in task 002); when `showLabel`, appends the status label text.

### Status token consumption (task 002 contract)
The `--status-*` custom properties are **bare HSL channel triplets**, so they MUST be consumed
wrapped in `hsl(...)`: `bg-[hsl(var(--status-<value>))]`. `<value>` is the `TaskStatus` union
literal (`todo|inprogress|inreview|done|cancelled`).

**CRITICAL — no template-literal class names.** Tailwind only generates an arbitrary-value class
when it sees the *complete literal string* in source. `bg-[hsl(var(--status-${status}))]` produces
NO CSS (the dot renders colourless). Use a static `Record<TaskStatus, string>` with full literal
class strings, exactly like `ConnectionStatusBadge`'s `statusConfig` map.

### SIBLING-ALIGN (rubric clause 9)
Sibling read (NOT edited; not in `files:`): `frontend/src/components/common/ConnectionStatusBadge.tsx`.
Conventions mirrored:
- A static `const ... : Record<Status, {...}>` config object mapping each status → its presentation
  (colour class + label), keyed by the status union literals. StatusBadge uses the same shape
  (`Record<TaskStatus, { dotClass: string; label: string }>`).
- `className` merged last via `cn(...)` from `@/lib/utils` (same import).
- Named function export `export function StatusBadge(...)`.
Justified divergences: ConnectionStatusBadge wraps a Tooltip and always shows its label;
StatusBadge has no tooltip and gates the label behind `showLabel` (per the contract — an 8px dot
with optional label, not a pill). It uses the `--status-*` HSL tokens rather than fixed
`bg-green-*`/`bg-blue-*` Tailwind colours, since theming via tokens is the whole point of this
workstream.

### TaskStatus import + labels
- `TaskStatus` is a **string union** (`"todo" | "inprogress" | ...`), not an enum — import as a
  type: `import type { TaskStatus } from 'shared/types';` (match how task components consume it).
- Labels: the existing i18n source (`t('status.*')` in `TaskCountPills`, namespace `projects`) does
  NOT map cleanly — keys are `todo|inProgress|review|done` (no `cancelled`; casing differs from the
  enum values `inprogress|inreview|cancelled`). To avoid coupling to a mismatched/incomplete key
  set, use English literals with a `// TODO(i18n): vk-swarm-node-ui-localize` comment, matching
  ConnectionStatusBadge's literal `label` convention.

### Full file contents (allowed_change: create)
```tsx
import type { TaskStatus } from 'shared/types';
import { cn } from '@/lib/utils';

interface StatusBadgeProps {
  status: TaskStatus;
  /** When true, append the status label text after the dot. */
  showLabel?: boolean;
  className?: string;
}

// Static literal class strings so Tailwind's content scan generates the
// arbitrary-value classes (a template-literal `--status-${status}` would not
// be emitted). Tokens (bare HSL triplets) come from task 002 — wrap in hsl().
// TODO(i18n): vk-swarm-node-ui-localize — labels are English literals.
const statusConfig: Record<TaskStatus, { dotClass: string; label: string }> = {
  todo: { dotClass: 'bg-[hsl(var(--status-todo))]', label: 'Todo' },
  inprogress: {
    dotClass: 'bg-[hsl(var(--status-inprogress))]',
    label: 'In Progress',
  },
  inreview: { dotClass: 'bg-[hsl(var(--status-inreview))]', label: 'In Review' },
  done: { dotClass: 'bg-[hsl(var(--status-done))]', label: 'Done' },
  cancelled: {
    dotClass: 'bg-[hsl(var(--status-cancelled))]',
    label: 'Cancelled',
  },
};

/**
 * Status indicator: an 8px coloured dot (driven by the --status-* tokens) with
 * an optional trailing label. Mirrors ConnectionStatusBadge's config-map +
 * cn() conventions.
 */
export function StatusBadge({
  status,
  showLabel = false,
  className,
}: StatusBadgeProps) {
  const config = statusConfig[status];

  return (
    <span
      className={cn('inline-flex items-center gap-1.5', className)}
    >
      <span
        className={cn('h-2 w-2 rounded-full shrink-0', config.dotClass)}
        aria-hidden="true"
      />
      {showLabel && (
        <span className="text-xs font-medium">{config.label}</span>
      )}
    </span>
  );
}
```

## Allowed moves
- ONLY: create `frontend/src/components/common/StatusBadge.tsx` with the contents above. No edits to
  `ConnectionStatusBadge` (sibling read only) or any consumer (placement is task 020).

## STOP triggers
- `frontend/src/components/common/StatusBadge.tsx` already exists (halt; reconcile, don't overwrite).
- `TaskStatus` is no longer the union `"todo" | "inprogress" | "inreview" | "done" | "cancelled"`
  in `shared/types` (halt — the `Record` keys would be wrong; re-derive from the current type).
- The `--status-*` tokens from task 002 are absent in `index.css` (halt — 002 not applied; dots
  would render colourless).

## Manual verification (record in decisions-ledger)
- `cd frontend && npx tsc --noEmit` → passes.
- `grep -E 'export function StatusBadge|export const StatusBadge' frontend/src/components/common/StatusBadge.tsx` → match.
- `grep 'bg-\[hsl(var(--status-' frontend/src/components/common/StatusBadge.tsx` → 5 literal matches
  (one per TaskStatus; confirms no template-literal interpolation).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 014` exits 0
