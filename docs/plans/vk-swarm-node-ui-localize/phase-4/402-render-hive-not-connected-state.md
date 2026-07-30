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

For each of `SwarmProjectsSection`, `NodeProjectsSection`, `SwarmLabelsSection`,
`SwarmTemplatesSection`, `NodeTemplatesSection`, and `frontend/src/pages/Nodes.tsx`:

- **Anchor:** the existing `) : error ? (` branch (or that file's equivalent error branch)
- **Change:** branch FIRST on `isHiveNotConfigured(error)` and render `<HiveNotConnected />`;
  keep the existing generic error branch as the `else`.

Sketch (adapt to each file's actual JSX shape — do not restructure the component):

```tsx
) : isHiveNotConfigured(error) ? (
  <HiveNotConnected />
) : error ? (
  /* ...existing generic error UI, unchanged... */
```

## Allowed moves

- Only the files in `files:`. Do not change any query hook's `retry`/`staleTime` behaviour.

## STOP triggers

- If a section has no `error` value in scope from its query hook — STOP and report which one;
  do not add a new query.
- If a section's error branch is shared with a different error presentation you would have to
  restructure — STOP rather than refactoring the component.
- Do NOT touch `remote-frontend/` — the hive is always "connected" to itself and must be
  unchanged (SC7).

## Manual verification (record in decisions-ledger)

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
