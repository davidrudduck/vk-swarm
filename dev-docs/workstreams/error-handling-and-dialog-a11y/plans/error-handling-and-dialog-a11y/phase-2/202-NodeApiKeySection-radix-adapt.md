---
id: "202"
phase: 2
title: "Update NodeApiKeySection for Radix dialog API"
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/NodeApiKeySection.tsx
  - remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: edit
covers_criteria: [SC3, SC6]
---
## Failing test (write first)
Covered by: `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` (36 existing tests).
Tests TS4, TS15, TS23 exercise dialog open/close/uncloseable behavior. After this task, all 36
tests must still pass — the Radix dialog preserves the same API surface.

## Change

Radix `DialogPrimitive.Root` does NOT accept `uncloseable` — it accepts only `open`,
`onOpenChange`, `defaultOpen`, `modal`, `children`. The `uncloseable` prop must move from
`<Dialog>` to `<DialogContent>` (which has it from task 201's rewrite).

- **File:** remote-frontend/src/components/swarm/NodeApiKeySection.tsx
- **Anchor:** line 419-424 (Dialog usage)
- **Before:**
```tsx
<Dialog
  open={showCreateDialog}
  onOpenChange={(open) => {
    if (!open && !createdSecret && !createMutation.isPending) closeDialog();
  }}
  uncloseable={!!createdSecret || createMutation.isPending}
>
  <DialogContent>
```
- **After:**
```tsx
<Dialog
  open={showCreateDialog}
  onOpenChange={(open) => {
    if (!open && !createdSecret && !createMutation.isPending) closeDialog();
  }}
>
  <DialogContent uncloseable={!!createdSecret || createMutation.isPending}>
```

This is the ONLY change. `uncloseable` moves from the Root to the Content element.

## Allowed moves
- Move `uncloseable` prop from `<Dialog>` to `<DialogContent>` in NodeApiKeySection.tsx
- No other changes

## STOP triggers
- If any of the 36 existing tests fail after the change (fix before proceeding)
- If the `uncloseable` behavior changes (close button hidden, Escape blocked, overlay click blocked)

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx
# Expected: all 36 tests pass
cd remote-frontend && npx tsc --noEmit
# Expected: no type errors
```

## Done when
- `uncloseable` prop moved from `<Dialog>` to `<DialogContent>`
- All 36 existing tests pass (SC6)
