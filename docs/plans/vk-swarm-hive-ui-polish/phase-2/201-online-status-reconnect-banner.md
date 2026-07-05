---
id: "201"
phase: 2
title: Create useOnlineStatus hook + reconnect banner in NormalLayout
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/lib/offline.ts
  - remote-frontend/src/components/layout/NormalLayout.tsx
irreversible: false
scope_test: "remote-frontend/src/lib/offline"
allowed_change: mixed
covers_criteria: [SC7]
---

## Failing test (write first)

Create `remote-frontend/src/lib/offline.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useOnlineStatus } from './offline';
import { renderHook, act } from '@testing-library/react';

describe('useOnlineStatus (SC7)', () => {
  beforeEach(() => {
    Object.defineProperty(navigator, 'onLine', {
      configurable: true,
      value: true,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns isOnline true with working values', () => {
    const { result } = renderHook(() => useOnlineStatus());
    expect(result.current.isOnline).toBe(true);
    expect(result.current.wasOffline).toBe(false);
    expect(result.current.lastOnlineAt).toBeNull();
  });

  it('sets isOnline false and wasOffline true on offline event', () => {
    const { result } = renderHook(() => useOnlineStatus());
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });
    expect(result.current.isOnline).toBe(false);
    expect(result.current.wasOffline).toBe(true);
  });

  it('sets isOnline true and lastOnlineAt on online event', () => {
    const { result } = renderHook(() => useOnlineStatus());
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });
    act(() => {
      window.dispatchEvent(new Event('online'));
    });
    expect(result.current.isOnline).toBe(true);
    expect(result.current.wasOffline).toBe(true);
    expect(result.current.lastOnlineAt).toBeInstanceOf(Date);
  });

  it('resets wasOffline when online->offline->online', () => {
    const { result } = renderHook(() => useOnlineStatus());
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });
    expect(result.current.wasOffline).toBe(true);
    act(() => {
      window.dispatchEvent(new Event('online'));
    });
    expect(result.current.wasOffline).toBe(true);
  });
});
```

## Change

### File: `remote-frontend/src/lib/offline.ts` (CREATE)

```ts
import { useState, useEffect, useCallback } from 'react';

interface OnlineStatus {
  isOnline: boolean;
  wasOffline: boolean;
  lastOnlineAt: Date | null;
}

export function useOnlineStatus(): OnlineStatus {
  const [isOnline, setIsOnline] = useState(navigator.onLine);
  const [wasOffline, setWasOffline] = useState(false);
  const [lastOnlineAt, setLastOnlineAt] = useState<Date | null>(null);

  const handleOnline = useCallback(() => {
    setIsOnline(true);
    setLastOnlineAt(new Date());
  }, []);

  const handleOffline = useCallback(() => {
    setIsOnline(false);
    setWasOffline(true);
  }, []);

  useEffect(() => {
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [handleOnline, handleOffline]);

  return { isOnline, wasOffline, lastOnlineAt };
}
```

### File: `remote-frontend/src/components/layout/NormalLayout.tsx` (EDIT — add reconnect banner)

- **Anchor:** the component body (~L4-L11)

Before:
```tsx
import { Outlet } from 'react-router-dom';
import { Navbar } from '@/components/layout/Navbar';
import { BottomNav } from '@/components/layout/BottomNav';

export function NormalLayout() {
  return (
    <>
      <Navbar />
      <div className="flex-1 min-h-0 overflow-hidden pb-14 sm:pb-0">
        <Outlet />
      </div>
      <BottomNav />
    </>
  );
}
```

After:
```tsx
import { Outlet } from 'react-router-dom';
import { Navbar } from '@/components/layout/Navbar';
import { BottomNav } from '@/components/layout/BottomNav';
import { useOnlineStatus } from '@/lib/offline';

export function NormalLayout() {
  const { isOnline } = useOnlineStatus();

  return (
    <>
      <Navbar />
      {!isOnline && (
        <div className="bg-amber-900/30 border-b border-amber-600/50 text-amber-200 text-sm text-center py-1.5">
          You're offline — changes will sync when reconnected
        </div>
      )}
      <div className="flex-1 min-h-0 overflow-hidden pb-14 sm:pb-0">
        <Outlet />
      </div>
      <BottomNav />
    </>
  );
}
```

## Allowed moves

- Create `remote-frontend/src/lib/offline.ts` with the exact code above.
- Create `remote-frontend/src/lib/offline.test.ts` with the exact code above.
- Edit `remote-frontend/src/components/layout/NormalLayout.tsx`: add `useOnlineStatus` import, add `const { isOnline } = useOnlineStatus()`, add the conditional reconnect banner between `<Navbar />` and the content div.
- Do NOT touch any other file.

## STOP triggers

- The NormalLayout.tsx Before text doesn't match. Verify with `git diff`.
- `@testing-library/react`'s `renderHook` is not available (it's a sub-export; if the vitest configuration doesn't resolve it, install `@testing-library/react-hooks` or use the `render` wrapper pattern instead — record in decisions ledger).
- The offline banner styling doesn't respect the Midnight Terminal palette. The amber colors are deliberate — they signal warning/offline state without being as alarming as red.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/offline" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 201` exits 0.