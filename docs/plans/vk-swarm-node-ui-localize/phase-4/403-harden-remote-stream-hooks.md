---
id: "403"
phase: 4
title: "Harden the four remote stream hooks for a node with no hive"
status: ready
depends_on: ["401"]
parallel: false
conflicts_with: []
files:
  - frontend/src/hooks/useNodeLogStream.ts
  - frontend/src/hooks/useDiffStream.ts
  - frontend/src/hooks/useRemoteConnectionStatus.ts
  - frontend/src/hooks/useAvailableNodes.ts
  - frontend/src/hooks/useAvailableNodes.test.ts
siblings:
  - frontend/src/hooks/useRemoteConnectionStatus.ts
irreversible: false
scope_test: "frontend/src/hooks"
allowed_change: mixed
covers_criteria: [SC6]
---

## Failing test (write first)

Create `frontend/src/hooks/useAvailableNodes.test.ts` asserting the hook surfaces a clean
disabled/empty result — not a thrown error — when the API responds with the hive-not-configured
error (503):

```typescript
import { describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import { ApiError } from '@/lib/api/utils';
import { useAvailableNodes } from './useAvailableNodes';

vi.mock('@/lib/api', () => ({
  tasksApi: { availableNodes: vi.fn() },
}));

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return React.createElement(QueryClientProvider, { client }, children);
}

describe('useAvailableNodes with no hive', () => {
  it('does not throw and reports no nodes when the server says HiveNotConfigured', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.availableNodes).mockRejectedValue(new ApiError('no hive', 503));

    const { result } = renderHook(() => useAvailableNodes('task-1'), { wrapper });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    // The consumer (CreateAttemptDialog) must be able to render: no throw,
    // and an empty node list rather than undefined-dereference.
    expect(result.current.nodes ?? []).toEqual([]);
  });
});
```

Adapt `result.current.nodes` to whatever shape the hook actually returns after your change (see
step 1) — but the assertion must be that a consumer can render, not merely that the hook settled.

## Sibling alignment (required reading before you write)

Read `frontend/src/hooks/useRemoteConnectionStatus.ts` first — it already branches on
`connectionInfo.direct_url` and is the in-repo model for degrading gracefully when remote data is
absent. The other three hooks must follow the same shape. Record any divergence in the ledger.

## Change

For each of the four hooks, make the hive-absent path a normal, quiet outcome:

1. **`useAvailableNodes.ts`** — treat a `isHiveNotConfigured(error)` failure as "no nodes
   available" and expose an empty list plus a boolean the dialog can read, rather than an error
   the caller must handle. `CreateAttemptDialog` must still render its local-attempt path.
2. **`useNodeLogStream.ts`** — when no hive is configured, do not open or retry a stream; return
   the existing empty-logs shape with no error set, so `ProcessLogsViewer` shows local logs.
3. **`useDiffStream.ts`** — same: no stream, no retry loop, empty diffs, no error. `DiffsPanel`
   and `useDiffSummary` must render local diffs.
4. **`useRemoteConnectionStatus.ts`** — ensure the not-configured case resolves to a definite
   disconnected status rather than an indefinite pending one, so `AttemptHeaderActions` renders.

Use `isHiveNotConfigured` from `@/lib/api/utils` (added in task 402) for the detection.

## Allowed moves

- Only the four hooks and the new test file. Do **not** modify `ProcessLogsViewer`,
  `DiffsPanel`, `AttemptHeaderActions`, or `CreateAttemptDialog` — if a consumer genuinely cannot
  render without a change, STOP and report; that is a plan gap, not an implementer decision.

## STOP triggers

- If making a hook degrade cleanly would require changing one of the four consumer components.
- If a hook currently has a retry/backoff loop whose removal would change behaviour when a hive
  IS configured — STOP; the hive-configured path must be untouched.
- If `isHiveNotConfigured` does not exist, task 402 has not run — STOP.

## Manual verification (record in decisions-ledger)

```bash
cd frontend && npx vitest run src/hooks
# Expected: the new useAvailableNodes test passes, existing hook tests still pass

cd frontend && npx tsc --noEmit && npm run lint
# Expected: clean

# Browser check on a node with NO hive configured (SC6), with an attempt open:
#   - ProcessLogsViewer renders local logs
#   - DiffsPanel renders local diffs
#   - AttemptHeaderActions renders a settled (not perpetually pending) status
#   - CreateAttemptDialog opens and can start a local attempt
#   - DevTools console: no unhandled rejection, no repeating retry errors
```

Record the console observation verbatim.

## Done when

- All four hooks return a clean, settled, empty result when no hive is configured.
- The four consumer components render on a hive-less node with no console errors.
- No behaviour changes when a hive IS configured.
- vitest, `tsc --noEmit`, and lint are clean.
