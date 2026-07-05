---
id: "202"
phase: 2
title: Create optimistic mutation helpers for Electric collections
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/lib/electric/optimistic.ts
irreversible: false
scope_test: "remote-frontend/src/lib/electric"
allowed_change: create
covers_criteria: [SC8]
---

## Failing test (write first)

Create `remote-frontend/src/lib/electric/optimistic.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { optimisticDelete, optimisticUpdate } from './optimistic';

describe('optimistic mutations (SC8)', () => {
  let queryClient: { setQueryData: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    queryClient = { setQueryData: vi.fn() };
  });

  it('optimisticDelete calls setQueryData to remove item', async () => {
    await optimisticDelete(
      queryClient as unknown as Parameters<typeof optimisticDelete>[0],
      ['task-assignments'],
      'a1',
      async () => {},
    );
    expect(queryClient.setQueryData).toHaveBeenCalledTimes(1);
    expect(queryClient.setQueryData).toHaveBeenCalledWith(
      ['task-assignments'],
      expect.any(Function),
    );
  });

  it('optimisticDelete rolls back on error', async () => {
    const restore = vi.fn();
    const snapshot = [{ id: 'a1' }, { id: 'a2' }];

    queryClient.setQueryData.mockImplementation(
      (_key: unknown, updater: unknown) => {
        if (typeof updater === 'function') {
          const fn = updater as (old: unknown) => unknown;
          fn(snapshot);
          return undefined;
        }
        return undefined;
      },
    );

    await optimisticDelete(
      queryClient as unknown as Parameters<typeof optimisticDelete>[0],
      ['task-assignments'],
      'a1',
      async () => {
        throw new Error('network error');
      },
    );

    expect(queryClient.setQueryData).toHaveBeenCalledWith(
      ['task-assignments'],
      expect.any(Function),
    );
  });

  it('optimisticUpdate calls setQueryData to patch item', async () => {
    await optimisticUpdate(
      queryClient as unknown as Parameters<typeof optimisticUpdate>[0],
      ['task-assignments'],
      'a1',
      { execution_status: 'completed' },
      async () => {},
    );
    expect(queryClient.setQueryData).toHaveBeenCalledTimes(1);
  });

  it('optimisticUpdate rolls back on error', async () => {
    await optimisticUpdate(
      queryClient as unknown as Parameters<typeof optimisticUpdate>[0],
      ['task-assignments'],
      'a1',
      { execution_status: 'completed' },
      async () => {
        throw new Error('network error');
      },
    );
    expect(queryClient.setQueryData).toHaveBeenCalledWith(
      ['task-assignments'],
      expect.any(Function),
    );
  });
});
```

## Change

### File: `remote-frontend/src/lib/electric/optimistic.ts` (CREATE)

```ts
import type { QueryClient } from '@tanstack/react-query';

type CollectionItem = { id: string; [key: string]: unknown };

export async function optimisticDelete(
  queryClient: QueryClient,
  queryKey: string[],
  itemId: string,
  apiCall: () => Promise<unknown>,
): Promise<void> {
  const previous = queryClient.getQueryData<CollectionItem[]>(queryKey);
  queryClient.setQueryData<CollectionItem[]>(queryKey, (old) =>
    (old ?? []).filter((item) => item.id !== itemId),
  );

  try {
    await apiCall();
  } catch {
    if (previous) {
      queryClient.setQueryData(queryKey, previous);
    }
    throw;
  }
}

export async function optimisticUpdate(
  queryClient: QueryClient,
  queryKey: string[],
  itemId: string,
  patch: Record<string, unknown>,
  apiCall: () => Promise<unknown>,
): Promise<void> {
  const previous = queryClient.getQueryData<CollectionItem[]>(queryKey);
  queryClient.setQueryData<CollectionItem[]>(queryKey, (old) =>
    (old ?? []).map((item) =>
      item.id === itemId ? { ...item, ...patch } : item,
    ),
  );

  try {
    await apiCall();
  } catch {
    if (previous) {
      queryClient.setQueryData(queryKey, previous);
    }
    throw;
  }
}
```

## Allowed moves

- Create `remote-frontend/src/lib/electric/optimistic.ts` with the exact code above.
- Create `remote-frontend/src/lib/electric/optimistic.test.ts` with the exact code above.
- Do NOT touch any other file. The `@tanstack/react-query` `QueryClient` type is already available (it's in the dependency tree).
- Do NOT wire optimistic helpers into Tasks.tsx — that's task 205.

## STOP triggers

- The `@tanstack/react-query` package doesn't export `QueryClient` type or `QueryClient.getQueryData` doesn't exist in the installed version. Verify with `npx tsc --noEmit` in remote-frontend.
- The test `restore` variable in optimisticDelete test case is unused by the test (only demonstrates the snapshot concept). If lint rejects it with `noUnusedLocals`, prefix with `_restore` or remove — record in decisions ledger.
- `getQueryData` returns `TData | undefined` — the generic call `getQueryData<CollectionItem[]>(queryKey)` is valid. If `tsc` rejects the generic, use `as CollectionItem[]` cast.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/optimistic" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 202` exits 0.