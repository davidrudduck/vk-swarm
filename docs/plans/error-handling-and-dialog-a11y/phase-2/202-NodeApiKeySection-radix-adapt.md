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
Covered by: `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` (28 existing tests).
Tests TS4, TS15, TS23 exercise dialog open/close/uncloseable behavior. After this task, all 28
tests must still pass — the Radix dialog preserves the same API surface.

## Change
- **File:** remote-frontend/src/components/swarm/NodeApiKeySection.tsx
- **Anchor:** import statement (line 16-22) and Dialog usage (lines 419-424)
- **Before:**
```typescript
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
```
- **After:** (same imports — no change needed if task 201 preserves all exports)

Check: verify `uncloseable` prop still works. The Dialog usage at line 424:
```tsx
<Dialog
  open={showCreateDialog}
  onOpenChange={(open) => {
    if (!open && !createdSecret && !createMutation.isPending) closeDialog();
  }}
  uncloseable={!!createdSecret || createMutation.isPending}
>
```
This must work unchanged with the Radix implementation from task 201.

If the Radix `Dialog` (Root) component does NOT accept `uncloseable` as a prop (Radix Root
accepts `open`, `onOpenChange`, `defaultOpen`, `modal`), then `uncloseable` must be moved to
`DialogContent` instead. In that case:

- **Before:** `<Dialog ... uncloseable={...}>` + `<DialogContent>`
- **After:** `<Dialog ...>` + `<DialogContent uncloseable={...}>`

This is the ONLY change expected. All other Dialog usage is unchanged.

## Allowed moves
- Move `uncloseable` prop from `Dialog` to `DialogContent` if Radix Root doesn't accept it
- No other changes to NodeApiKeySection.tsx

## STOP triggers
- If any of the 28 existing tests fail after the change (fix before proceeding)
- If the `uncloseable` behavior changes (close button hidden, Escape blocked, overlay click blocked)

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx
# Expected: all 28 tests pass
cd remote-frontend && npx tsc --noEmit
# Expected: no type errors
```

## Done when
- NodeApiKeySection renders correctly with Radix dialog
- `uncloseable` prop works (close button hidden during secret reveal, Escape blocked)
- All 28 existing tests pass (SC6)
