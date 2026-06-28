---
id: "011"
phase: 2
title: VKSLogo wordmark → font-wordmark (Chivo Mono)
status: ready
depends_on: ["004"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/VKSLogo.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
N/A — covered by manual verification (className swap; greppable assertions below).

## Change

The VKSLogo wordmark currently renders in `font-code` (JetBrains Mono). The brand wordmark should
use `font-wordmark` (Chivo Mono — the `fontFamily` alias added by task 004). Swap `font-code` →
`font-wordmark`.

### File: `frontend/src/components/VKSLogo.tsx`

**Anchor — the `cn(...)` className string. It appears TWICE, byte-for-byte identical** (line ~22 in
`VKSLogo`, line ~50 in `VKSIcon`). BOTH are the brand mark, so BOTH must change (the verification
`grep 'font-code' → no match` only passes if both are swapped). Because the two strings are
identical, an Edit on one is non-unique — use `replace_all` (or apply both Before/After blocks).

- Before (both occurrences):
```tsx
        'font-code font-bold tracking-tight select-none',
```
- After (both occurrences):
```tsx
        'font-wordmark font-bold tracking-tight select-none',
```

No other change. Leave the `text-primary`/`text-foreground` spans, the responsive `sm:hidden` /
`hidden sm:inline` wrappers, props, and `aria-label="VK-Swarm"` UNCHANGED.

## Allowed moves
- ONLY: replace `font-code` with `font-wordmark` in the two `cn(...)` className literals (one in
  `VKSLogo`, one in `VKSIcon`). Do not touch any other class, span, prop, or file.

## STOP triggers
- The className string differs materially from the Before text (re-grep `font-code font-bold
  tracking-tight select-none`; if absent, halt — the file changed since decompose).
- `font-code` appears anywhere other than the two identical brand-mark className strings (halt;
  reconcile — `replace_all` would catch an unintended occurrence).
- `font-wordmark` already present (halt — partly migrated).

## Manual verification (record in decisions-ledger)
- `grep 'font-wordmark' frontend/src/components/VKSLogo.tsx` → match (both occurrences).
- `grep 'font-code' frontend/src/components/VKSLogo.tsx` → no match.
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 011` exits 0
