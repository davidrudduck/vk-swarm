---
id: "203"
phase: 2
title: Create sync status indicator in Navbar
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: [204]
files:
  - remote-frontend/src/lib/electric/sync-status.ts
  - remote-frontend/src/lib/electric/sync-status.test.ts
  - remote-frontend/src/components/layout/Navbar.tsx
irreversible: false
scope_test: "remote-frontend/src/lib/electric"
allowed_change: mixed
covers_criteria: [SC9]
---

## Failing test (write first)

Create `remote-frontend/src/lib/electric/sync-status.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSyncStatus, getSyncStatus } from './sync-status';

describe('getSyncStatus function', () => {
  it('returns synced when last update < 30s ago', () => {
    expect(getSyncStatus(Date.now() - 10_000)).toBe('synced');
  });

  it('returns reconnecting when 30-60s since last update', () => {
    expect(getSyncStatus(Date.now() - 45_000)).toBe('reconnecting');
  });

  it('returns disconnected when > 60s since last update', () => {
    expect(getSyncStatus(Date.now() - 90_000)).toBe('disconnected');
  });

  it('returns synced when lastUpdateAt is null (initial state)', () => {
    expect(getSyncStatus(null)).toBe('synced');
  });
});

describe('useSyncStatus hook', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(navigator, 'onLine', {
      configurable: true,
      value: true,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('starts as synced', () => {
    const { result } = renderHook(() => useSyncStatus());
    expect(result.current.syncStatus).toBe('synced');
  });

  it('markSynced updates lastUpdateAt in ref', () => {
    const { result } = renderHook(() => useSyncStatus());
    act(() => {
      result.current.markSynced();
    });
    // status stays 'synced' because timestamp is fresh
    expect(result.current.syncStatus).toBe('synced');
  });

  it('goes to disconnected after 60s without markSynced', () => {
    const { result } = renderHook(() => useSyncStatus());
    vi.advanceTimersByTime(61_000);
    expect(result.current.syncStatus).toBe('disconnected');
  });

  it('markSynced resets back to synced after disconnect', () => {
    const { result } = renderHook(() => useSyncStatus());
    vi.advanceTimersByTime(61_000);
    expect(result.current.syncStatus).toBe('disconnected');
    act(() => {
      result.current.markSynced();
    });
    expect(result.current.syncStatus).toBe('synced');
  });

  it('goes offline on window offline event', () => {
    const { result } = renderHook(() => useSyncStatus());
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });
    expect(result.current.syncStatus).toBe('disconnected');
  });

  it('cleans up interval on unmount', () => {
    const { unmount } = renderHook(() => useSyncStatus());
    const clearSpy = vi.spyOn(global, 'clearInterval');
    unmount();
    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore();
  });
});
```

## Change

### File: `remote-frontend/src/lib/electric/sync-status.ts` (CREATE)

```ts
import { useState, useEffect, useRef, useCallback } from 'react';

export type SyncStatus = 'synced' | 'reconnecting' | 'disconnected';

export function getSyncStatus(lastUpdateAt: number | null): SyncStatus {
  if (lastUpdateAt === null) return 'synced';
  const elapsed = Date.now() - lastUpdateAt;
  if (elapsed < 30_000) return 'synced';
  if (elapsed < 60_000) return 'reconnecting';
  return 'disconnected';
}

export function useSyncStatus() {
  const lastUpdateRef = useRef<number>(Date.now());
  const [syncStatus, setSyncStatus] = useState<SyncStatus>('synced');

  const markSynced = useCallback(() => {
    lastUpdateRef.current = Date.now();
    setSyncStatus('synced');
  }, []);

  useEffect(() => {
    const tick = () => {
      const status = getSyncStatus(lastUpdateRef.current);
      setSyncStatus(status);
    };

    const handleOnline = () => {
      lastUpdateRef.current = Date.now();
      setSyncStatus('synced');
    };
    const handleOffline = () => setSyncStatus('disconnected');

    const interval = setInterval(tick, 10_000);
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      clearInterval(interval);
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return { syncStatus, markSynced };
}
```

### File: `remote-frontend/src/components/layout/Navbar.tsx` (EDIT — add sync status dot with live hook)

**Anchor 1 — imports:** Add after the existing imports:

```tsx
import { useSyncStatus } from '@/lib/electric/sync-status';
```

**Anchor 2 — component body, after `const location = useLocation();`:**

Before:
```tsx
  const location = useLocation();

  const handleLogout = async () => {
```

After:
```tsx
  const location = useLocation();

  const { syncStatus } = useSyncStatus();
  const syncColor = {
    synced: 'bg-green-500',
    reconnecting: 'bg-yellow-500',
    disconnected: 'bg-red-500',
  };

  const handleLogout = async () => {
```

**Anchor 3 — the "VK Swarm" link in the header:**

Before:
```tsx
            <Link to="/nodes" className="text-foreground font-semibold">
              VK Swarm
            </Link>
```

After:
```tsx
            <Link to="/nodes" className="text-foreground font-semibold flex items-center gap-2">
              VK Swarm
              <span
                className={`inline-block w-2 h-2 rounded-full ${syncColor[syncStatus]}`}
                title={`Sync: ${syncStatus}`}
                aria-label={`Sync status: ${syncStatus}`}
              />
            </Link>
```

**Tasks.tsx integration — call `markSynced` from a data consumer:**

In `remote-frontend/src/pages/Tasks.tsx` (task 103's output), add one more edit to wire the hook into the first data consumer. No additional task file needed — the implementer adds this as part of the Navbar change.

At the top of `TasksBoard`, after the hook declarations, call `markSynced` whenever any of the 5 live queries produce data:

```tsx
  // After all useLiveQuery calls, add:
  const { markSynced } = useSyncStatus();
  useEffect(() => { markSynced(); }, [assignments, nodes, projects]);
```

Import addition in Tasks.tsx:
```tsx
import { useSyncStatus } from '@/lib/electric/sync-status';
import { useEffect } from 'react';
```

(If `useEffect` is not already imported, add it to the React import.)

## Allowed moves

- Create `remote-frontend/src/lib/electric/sync-status.ts` with the exact code above (exports `getSyncStatus`, `useSyncStatus`, `SyncStatus`).
- Create `remote-frontend/src/lib/electric/sync-status.test.ts` with the exact test code above.
- Edit `remote-frontend/src/components/layout/Navbar.tsx` at exactly 3 anchors: (1) add `useSyncStatus` import, (2) add `const { syncStatus } = useSyncStatus()` + `syncColor` map after `useLocation()`, (3) wrap the VK Swarm link text with sync dot span.
- Edit `remote-frontend/src/pages/Tasks.tsx` (which was created/modified by task 103): add `import { useSyncStatus }` + `import { useEffect }` and the `const { markSynced } = useSyncStatus()` + `useEffect` call after the live query hooks.
- Do NOT touch any other file.

## STOP triggers

- The Navbar.tsx Before text doesn't match exactly. Verify with `git diff`.
- Task 103 is not complete (Tasks.tsx doesn't exist in the task 103 shape yet). Verify.
- `@testing-library/react`'s `renderHook` is not available — use `@testing-library/react-hooks` or the `render` wrapper pattern. Record in decisions ledger.
- `useFakeTimers` conflicts with `useEffect` cleanup tests in vitest. If the unmount/clearInterval test flakes, split it into a separate file with `@vitest-environment node`. Record.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/sync-status" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 203` exits 0.