---
id: "006"
phase: 2
title: TaskCard typography/meta fidelity + TaskCardHeader title
status: ready
depends_on: ["002", "005"]
parallel: false
conflicts_with: ["005"]
files:
  - frontend/src/components/tasks/TaskCard.tsx
  - frontend/src/components/tasks/AllProjectsTaskCard.tsx
  - frontend/src/components/tasks/TaskCardHeader.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
N/A — typography/meta fidelity; covered by manual verification (greppable class assertions +
`tsc --noEmit`) and a manual browser check that descriptions still render and truncate via CSS.

## Change

Six sub-edits. READ each file and confirm the Before text before editing.

> **SC7 "consumed" clause:** task 002 defines the `--border-strong` token but, per the gap analysis,
> no component consumes it. Sub-edit 6 below adds a strong hover border to the `TaskCard` root so the
> token is consumed (satisfying SC7's "defined **and** consumed" requirement). This is why `002` is
> now in `depends_on` — the token must exist before this card can reference it.

### Sub-edit 1 — `frontend/src/components/tasks/TaskCardHeader.tsx` (title className, ~line 52)

Change `font-light text-sm` → `font-medium text-base`; keep the rest identical.

- Before:
```typescript
        className={`flex-1 min-w-0 line-clamp-2 font-light text-sm ${titleClassName ?? ''}`}
```
- After:
```typescript
        className={`flex-1 min-w-0 line-clamp-2 font-medium text-base ${titleClassName ?? ''}`}
```

### Sub-edit 2 — `frontend/src/components/tasks/TaskCard.tsx` (description `<p>`, ~line 268)

Change `text-xs` → `text-sm`.

- Before:
```typescript
            className="text-xs text-muted-foreground truncate"
```
- After:
```typescript
            className="text-sm text-muted-foreground truncate"
```

### Sub-edit 3 — `frontend/src/components/tasks/TaskCard.tsx` (node tag `<span>`, ~line 278)

Add `font-mono`.

- Before:
```typescript
              <span className="text-xs text-muted-foreground shrink-0">
```
- After:
```typescript
              <span className="text-xs font-mono text-muted-foreground shrink-0">
```

### Sub-edit 4 — `frontend/src/components/tasks/TaskCard.tsx` (merged-attempt CheckCircle, ~line 245)

Change `text-green-500` → `text-success` on the merged/success indicator only. Leave the Loader2
`text-blue-500` spinner (~line 242) UNCHANGED — the spec calls out only the merged/success indicator.

- Before:
```typescript
                <CheckCircle className="h-4 w-4 text-green-500" />
```
- After:
```typescript
                <CheckCircle className="h-4 w-4 text-success" />
```

### Sub-edit 5 — Remove JS pre-truncation (behaviour change)

`truncateDescription` is defined in BOTH files but the two definitions are NOT identical — handle each
per its actual behaviour. The goal: pass the raw (UNtruncated) description string to the element that
already has CSS `truncate`, relying on CSS for the visual cap. Document the exact approach in the
ledger.

**5a — `frontend/src/components/tasks/TaskCard.tsx` (function ~lines 49–64; call site ~line 196).**
This definition strips leading markdown headers and collapses whitespace — that cleaning is VALUABLE
and must be KEPT. Drop ONLY the length cap. Then remove the now-unused `maxLength` parameter (the call
site already omits it; leaving an unused parameter will fail the `tsc --noEmit` gate).

- Before (function):
```typescript
function truncateDescription(
  description: string | null | undefined,
  maxLength: number = 80
): string | null {
  if (!description) return null;
  // Strip leading markdown headers and empty lines to show meaningful content
  const cleaned = description
    .replace(/^\s+/, '')             // trim leading whitespace first (handles \n before headers)
    .replace(/^(#{1,6} [^\n]*\n?)+/, '') // remove leading markdown headers (require space after # to avoid eating #hashtags)
    .replace(/^\s+/, '')             // trim any whitespace exposed after header removal
    .replace(/\s+/g, ' ')           // collapse internal newlines to spaces
    .trim();
  if (!cleaned) return null;
  if (cleaned.length <= maxLength) return cleaned;
  return `${cleaned.substring(0, maxLength)}...`;
}
```
- After (function — keep cleaning, drop cap + unused param; rename to reflect new role is optional but
  the call site MUST stay valid):
```typescript
function cleanDescription(
  description: string | null | undefined
): string | null {
  if (!description) return null;
  // Strip leading markdown headers and empty lines to show meaningful content
  const cleaned = description
    .replace(/^\s+/, '')             // trim leading whitespace first (handles \n before headers)
    .replace(/^(#{1,6} [^\n]*\n?)+/, '') // remove leading markdown headers (require space after # to avoid eating #hashtags)
    .replace(/^\s+/, '')             // trim any whitespace exposed after header removal
    .replace(/\s+/g, ' ')           // collapse internal newlines to spaces
    .trim();
  if (!cleaned) return null;
  return cleaned;
}
```
- Before (call site, ~line 196):
```typescript
  const truncatedDesc = useMemo(() => truncateDescription(task.description), [task.description]);
```
- After (call site — update the function name; the CSS `truncate` on the `<p>` does the visual cap):
```typescript
  const truncatedDesc = useMemo(() => cleanDescription(task.description), [task.description]);
```
  (If you prefer to keep the name `truncateDescription`, that is acceptable — but the
  variable/element wiring and the dropped cap/param are mandatory. Either way the raw cleaned string
  flows to the `<p className="... truncate">`.)

**5b — `frontend/src/components/tasks/AllProjectsTaskCard.tsx` (function ~lines 23–30; call site ~line 75).**
This definition is a PLAIN length cap with NO markdown cleaning — nothing is lost by removing it
entirely. Delete the function and pass `task.description` straight to the `<p>` (re-guarding on
`task.description`).

- Before (function):
```typescript
function truncateDescription(
  description: string | null | undefined,
  maxLength: number = 40
): string | null {
  if (!description) return null;
  if (description.length <= maxLength) return description;
  return `${description.substring(0, maxLength)}...`;
}
```
- After: DELETE the function entirely.

- Before (call site, ~line 75):
```typescript
  // Truncated description for compact view
  const truncatedDesc = truncateDescription(task.description, 40);
```
- After (call site — bind the raw description; the existing `{truncatedDesc && …}` guard at ~line 141
  then short-circuits on null/empty, and the `<p className="... truncate">` does the visual cap):
```typescript
  // Description for compact view (CSS truncate handles the visual cap)
  const truncatedDesc = task.description ?? null;
```

(Divergence-in-handling between 5a and 5b — TaskCard keeps the markdown cleaner, AllProjects drops a
pure length-cap helper — MUST be recorded in the decisions ledger.)

### Sub-edit 6 — `frontend/src/components/tasks/TaskCard.tsx` (card root hover border, ~line 212) — consumes `--border-strong` (SC7)

The card root is the `KanbanCard` element (`~lines 199–216`); its `className` is built with `cn(...)`
and flows down to the underlying `<Card>` (confirmed: `KanbanCard` passes `className` to `<Card>` in
`frontend/src/components/ui/shadcn-io/kanban/index.tsx`). Add a strong hover border by appending
`hover:border-[hsl(var(--border-strong))]` to the `transition-shadow duration-150 hover:shadow-md`
line in that `cn(...)` block. The token is a **bare HSL channel triplet** (task 002 —
`--border-strong: 240 10% 16%;` dark / `214 20% 80%;` light), so it MUST be wrapped in `hsl(...)`,
and the literal class string must appear verbatim in source (no template-literal interpolation) for
Tailwind's content scan to emit it.

- Before (~line 212, inside the `KanbanCard` `className={cn(` call):
```typescript
        'transition-shadow duration-150 hover:shadow-md',
```
- After:
```typescript
        'transition-shadow duration-150 hover:shadow-md hover:border-[hsl(var(--border-strong))]',
```

**Rendered-extent note (document in ledger, NOT a regression):** the base `<Card>` primitive has no
full border; `KanbanCard` adds only `border-b` (`'p-3 outline-none border-b flex-col space-y-2'`).
So `hover:border-[hsl(var(--border-strong))]` recolours the **bottom edge** on hover (the only bordered
edge), not a full box border. This still *consumes* `--border-strong` (satisfying SC7) and gives the
card a stronger hover affordance; do not add `border` / change `border-b` to chase a full-box border —
that is out of this task's scope.

## Allowed moves
- ONLY the six sub-edits above. No status-strip change (that is task 005). No new component. Do not
  add a full `border` class to the card root (sub-edit 6 only recolours the existing `border-b` on
  hover — see its rendered-extent note).
- Note (out of scope, do NOT change here): `AllProjectsTaskCard.tsx` carries the same
  `text-green-500` CheckCircle (~line 132) and `text-xs` description (~line 143) that this task changes
  only in `TaskCard.tsx`. The spec scopes sub-edits 2 and 4 to `TaskCard.tsx`; leave the
  `AllProjectsTaskCard.tsx` equivalents UNCHANGED and flag the asymmetry in the ledger.

## STOP triggers
- ANY Before text above differs from the real file when you read it (halt + report).
- The `truncateDescription` call sites are more complex than described — i.e. NOT the single
  `useMemo` (TaskCard ~line 196) and the single direct call (AllProjects ~line 75) — or there are
  additional call sites elsewhere (halt + report).

## Manual verification (record in decisions-ledger)
- `grep -- 'font-medium text-base' frontend/src/components/tasks/TaskCardHeader.tsx` → match.
- `grep -- 'text-sm text-muted-foreground truncate' frontend/src/components/tasks/TaskCard.tsx` → match.
- `grep -- 'font-mono' frontend/src/components/tasks/TaskCard.tsx` → match on the node-tag span.
- `grep -- 'CheckCircle className="h-4 w-4 text-success"' frontend/src/components/tasks/TaskCard.tsx` → match.
- `grep 'hsl(var(--border-strong))' frontend/src/components/tasks/TaskCard.tsx` → match (sub-edit 6
  consumes `--border-strong`, satisfying SC7's defined-and-consumed clause).
- `grep -c 'truncateDescription' frontend/src/components/tasks/AllProjectsTaskCard.tsx` → 0 (function removed).
- `cd frontend && npx tsc --noEmit` → passes (no unused-parameter / unused-symbol error).
- Manual browser check: task descriptions still render in both card variants and visually truncate via
  CSS (`truncate`) rather than JS substring.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 006` exits 0
