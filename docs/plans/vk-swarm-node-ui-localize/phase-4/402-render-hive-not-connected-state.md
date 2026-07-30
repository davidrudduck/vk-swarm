---
id: "402"
phase: 4
title: "Render an explicit not-connected-to-a-hive state across the swarm surfaces"
status: ready
depends_on: ["401", "104"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/swarm/HiveNotConnected.tsx
  - frontend/src/components/swarm/HiveNotConnected.test.tsx
  - frontend/src/lib/api/utils.ts
  - frontend/src/components/swarm/SwarmProjectsSection.tsx
  - frontend/src/components/swarm/NodeProjectsSection.tsx
  - frontend/src/components/swarm/SwarmLabelsSection.tsx
  - frontend/src/components/swarm/SwarmTemplatesSection.tsx
  - frontend/src/components/swarm/NodeTemplatesSection.tsx
  - frontend/src/pages/Nodes.tsx
  - frontend/src/components/swarm/index.ts
siblings:
  - frontend/src/components/ui/alert.tsx
irreversible: false
scope_test: "frontend/src/components/swarm"
allowed_change: mixed
covers_criteria: [SC2, SC4]
---

## Failing test (write first)

Create `frontend/src/components/swarm/HiveNotConnected.test.tsx`:

```typescript
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { HiveNotConnected } from './HiveNotConnected';
import { isHiveNotConfigured } from '@/lib/api/utils';
import { ApiError } from '@/lib/api/utils';

describe('HiveNotConnected', () => {
  it('renders an explicit not-connected message', () => {
    render(<HiveNotConnected />);
    expect(screen.getByText(/not connected to a hive/i)).toBeInTheDocument();
  });
});

describe('isHiveNotConfigured', () => {
  it('is true for a 503 ApiError', () => {
    expect(isHiveNotConfigured(new ApiError('nope', 503))).toBe(true);
  });

  it('is false for other errors', () => {
    expect(isHiveNotConfigured(new ApiError('bad', 400))).toBe(false);
    expect(isHiveNotConfigured(new Error('boom'))).toBe(false);
    expect(isHiveNotConfigured(null)).toBe(false);
  });
});
```

## Sibling alignment (required reading before you write)

Read `frontend/src/components/ui/alert.tsx` and the error branch of
`frontend/src/components/swarm/SwarmProjectsSection.tsx` (~line 212, `) : error ? (`). The new
component must use the same `Alert` primitive and the same `t(...)` i18n call convention those
use. Record any divergence in the decisions-ledger.

## Change

### 1. Add the detector — `frontend/src/lib/api/utils.ts`

- **Anchor:** immediately after the `ApiError` class declaration
- **After:** append:

```typescript
/**
 * True when an error is the server's "this node is not connected to a hive"
 * response (ApiError::HiveNotConfigured -> HTTP 503).
 */
export function isHiveNotConfigured(err: unknown): boolean {
  return err instanceof ApiError && err.status === 503;
}
```

### 2. Create `frontend/src/components/swarm/HiveNotConnected.tsx`

A presentational component using the shared `Alert` primitive, with a translated message whose
default English text contains "not connected to a hive" and explains that swarm management lives
on the hive. Export it from `frontend/src/components/swarm/index.ts` alongside the existing
section exports.

### 3. Wire the five sections and the Nodes page

The six targets do NOT share one shape. Decomposition checked each; use the exact variable named
below — do not assume `error` is in scope.

| File | Error value in scope | Insert the new branch |
|---|---|---|
| `SwarmProjectsSection.tsx` | `error` (destructured) | before `) : error ? (` |
| `SwarmLabelsSection.tsx` | `error` (destructured) | before `) : error ? (` |
| `SwarmTemplatesSection.tsx` | `error` (destructured) | before `) : error ? (` |
| `NodeTemplatesSection.tsx` | **`swarmTemplatesError` — must be added** (see below) | before `) : error ? (` at line 156 |
| `NodeProjectsSection.tsx` | **`nodesError`** (aliased at line 104: `error: nodesError,`) | before its error branch at line 282 |
| `pages/Nodes.tsx` | **none — only `isError`** (boolean) | see below |

For the first five, branch FIRST on the hive case and keep the existing generic error branch as
the `else` (adapt to each file's JSX; do not restructure the component):

```tsx
) : isHiveNotConfigured(error) ? (
  <HiveNotConnected />
) : error ? (
  /* ...existing generic error UI, unchanged... */
```

**`NodeTemplatesSection.tsx` is a trap — do NOT use its `error`.** That value belongs to the
LOCAL templates query (`templatesApi.list()`, lines 51-60), which SUCCEEDS on a hive-less node. The
hive-facing query is `useSwarmTemplates` at lines 63-66, whose error is not currently destructured.
Wiring the local error would leave the section rendering local templates and never showing the
disconnected state — silently failing SC4. Required change:

- **Anchor:** lines 63-66
- **Before:**
```tsx
  const { data: swarmTemplates = [] } = useSwarmTemplates({
```
- **After:**
```tsx
  const { data: swarmTemplates = [], error: swarmTemplatesError } = useSwarmTemplates({
```
- Then insert `) : isHiveNotConfigured(swarmTemplatesError) ? (<HiveNotConnected />` BEFORE the
  existing `) : error ? (` branch at line 156. Leave that branch and its "Failed to load local
  templates" copy exactly as they are — it still handles a genuine local failure.

For `pages/Nodes.tsx` the query destructure must gain `error` first:

- **Anchor:** lines 14-18
- **Before:**
```tsx
  const {
    data: nodes = [],
    isLoading: nodesLoading,
    isError,
  } = useQuery({
```
- **After:**
```tsx
  const {
    data: nodes = [],
    isLoading: nodesLoading,
    isError,
    error,
  } = useQuery({
```

- **Anchor:** line 38, the `) : isError ? (` branch
- **Before:**
```tsx
      ) : isError ? (
        <p className="text-muted-foreground">Failed to load nodes.</p>
```
- **After:**
```tsx
      ) : isHiveNotConfigured(error) ? (
        <HiveNotConnected />
      ) : isError ? (
        <p className="text-muted-foreground">Failed to load nodes.</p>
```

Leave the existing `!orgId` branch ("Nodes are a swarm feature. Connect a hive server to get
started.") alone — it fires earlier and covers a different case (no organizations at all).

## Allowed moves

- Only the files in `files:`. Do not change any query hook's `retry`/`staleTime` behaviour.

## STOP triggers

- If a section's error value is named something other than the table above says — STOP and
  report; do not guess. In particular, never wire a LOCAL query's error to `HiveNotConnected`:
  a local query succeeds on a hive-less node, so the disconnected state would never render. (Adding `error` to the `Nodes.tsx` destructure IS authorised above; adding
  a NEW query anywhere is not.)
- If a section's error branch is shared with a different error presentation you would have to
  restructure — STOP rather than refactoring the component.
- Do NOT touch `remote-frontend/` — the hive is always "connected" to itself and must be
  unchanged (SC7).

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cd frontend && npx vitest run src/components/swarm
# Expected: the new HiveNotConnected tests pass

cd frontend && npx tsc --noEmit && npm run lint
# Expected: clean

# Browser check with NO hive configured (SC4):
#   open /settings/swarm  -> every section shows "not connected to a hive"
#   open /nodes           -> same
#   DevTools console      -> no unhandled rejection; no infinite spinner
# Browser check WITH a hive configured (SC2):
#   open /settings/swarm  -> real swarm projects/labels/templates render
#   Network tab           -> zero 404s
```

Record both browser observations verbatim (status codes seen in the Network tab).

## Done when

- `isHiveNotConfigured` and `HiveNotConnected` exist and are unit-tested.
- All five swarm sections and the Nodes page render the not-connected state on a hive-less node.
- With a hive present, the same screens render live data with no 404s.
- vitest, `tsc --noEmit`, and lint are clean.
