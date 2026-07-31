---
id: "403"
phase: 4
title: "Harden the four remote stream hooks for a node with no hive"
status: ready
depends_on: ["401", "402"]
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

## Amendments (ORCHESTRATOR, pre-dispatch — verified facts, DICTATED)

**J1 — all four hooks exist; no STOP on missing anchors.** `useNodeLogStream.ts`, `useDiffStream.ts`,
`useRemoteConnectionStatus.ts`, `useAvailableNodes.ts` are all present under `frontend/src/hooks/`.

**J2 — `scope_test: "frontend/src/hooks"` is a REAL gate here, unlike earlier tasks.** That directory
currently has 6 test files and **68 tests, all passing** (verified on a clean tree), and NONE of the
8 baseline-red files (F-2026-07-31-01..03) live there. So the scope must be green BOTH before and
after your change. If any of those 68 tests breaks, that is a regression you caused — not
pre-existing debt. (The orchestrator runs the gate with
`WAI_TEST_CMD='(s={scope}; cd frontend && npx vitest run "${s#frontend/}")'` because the gate
otherwise invokes vitest from the repo root, where it is not installed.)

**J3 — discriminate on status 503, never the message.** Task 402 shipped
`isHiveNotConfigured(err)` in `frontend/src/lib/api/utils.ts` — `err instanceof ApiError &&
err.status === 503`. REUSE it; do not write a second detector, and do not match on the message text
(`"HiveNotConfigured: This node is not connected to a hive"` is a rendering detail, not a contract).

**J4 — the retry context you are hardening against, measured rather than assumed.** The global
`QueryClient` (`frontend/src/main.tsx:10-17`) sets only `staleTime` and `refetchOnWindowFocus` — there
is **no `retry` override**, so TanStack Query's default `retry: 3` applies and retries on ANY thrown
error without inspecting status. A hive-absent node therefore retries every hive-proxy query 3× with
backoff before settling. This was true before task 401 as well (the old 400 retried identically), so
it is pre-existing — but it is exactly what SC6 asks you to harden. `useAvailableNodes.ts:12-15`
currently has no `retry` override and gates only on `enabled: options?.enabled !== false && !!taskId`.

**J5 — do not "fix" the baseline.** The full frontend suite is red at baseline outside `hooks/`
(8 files / 15 tests). Leave those alone; report that the same 8 still fail with no new entrant.

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
2. **`useNodeLogStream.ts` — STOP AND REPORT; do not attempt this one.** Decomposition found it
   does not fit the mechanical rule and the plan will not guess:
   - It bypasses the API client entirely, calling `fetch('/v1/nodes/assignments/${id}/connection-info')`
     directly (lines 77-85) and throwing a plain `new Error(...)`. `isHiveNotConfigured` only
     matches `ApiError`, so it would be **always false** here.
   - `/v1/...` is a HIVE path. On a node with no hive that request reaches the NODE server, which
     has no `/v1` route, so it 404s — it never produces the 503 this workstream introduces.
   - Switching it to `tasksApi.streamConnectionInfo` is not a literal edit either: that helper
     takes a **task id**, and this hook is given an **assignment id**.

   Resolving this means either threading a task id from `ProcessLogsViewer` (a file NOT in
   `files:`) or deciding what a node-local log stream should do — a design decision above an
   implementer's pay grade. **Report this to the orchestrator and leave the file untouched.**
3. **`useDiffStream.ts`** — same: no stream, no retry loop, empty diffs, no error. `DiffsPanel`
   and `useDiffSummary` must render local diffs.
3b. **`useRemoteConnectionStatus.ts`** — ensure the not-configured case resolves to a definite
   disconnected status rather than an indefinite pending one, so `AttemptHeaderActions` renders.

Use `isHiveNotConfigured` from `@/lib/api/utils` (added in task 402) for the detection.

## Allowed moves

- Only `useAvailableNodes.ts`, `useDiffStream.ts`, `useRemoteConnectionStatus.ts`, and the new
  test file. `useNodeLogStream.ts` stays in `files:` only so a STOP report can cite it — do not
  edit it (see item 2). Do **not** modify `ProcessLogsViewer`,
  `DiffsPanel`, `AttemptHeaderActions`, or `CreateAttemptDialog` — if a consumer genuinely cannot
  render without a change, STOP and report; that is a plan gap, not an implementer decision.

## STOP triggers

- If making a hook degrade cleanly would require changing one of the four consumer components.
- If a hook currently has a retry/backoff loop whose removal would change behaviour when a hive
  IS configured — STOP; the hive-configured path must be untouched.
- If `isHiveNotConfigured` does not exist, task 402 has not run — STOP.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

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

- Three hooks (`useAvailableNodes`, `useDiffStream`, `useRemoteConnectionStatus`) return a clean,
  settled, empty result when no hive is configured.
- `useNodeLogStream` is reported back unmodified with the reason from item 2.
- The four consumer components render on a hive-less node with no console errors.
- No behaviour changes when a hive IS configured.
- vitest, `tsc --noEmit`, and lint are clean.
