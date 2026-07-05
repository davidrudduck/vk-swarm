---
id: "204"
phase: 2
title: Create offline mutation queue with idb-keyval
status: ready
depends_on: ["200", "203"]
parallel: false
conflicts_with: [203]
files:
  - remote-frontend/package.json
  - remote-frontend/src/lib/mutation-queue.ts
  - remote-frontend/src/components/layout/Navbar.tsx
irreversible: false
scope_test: "remote-frontend/src/lib/mutation-queue"
allowed_change: mixed
covers_criteria: [SC10]
---

## Failing test (write first)

Create `remote-frontend/src/lib/mutation-queue.test.ts`:

```ts
// @vitest-environment node
import { describe, it, expect, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('mutation queue module (SC10)', () => {
  it('exports enqueueMutation function', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export async function enqueueMutation');
    expect(source).toContain('idb-keyval');
    expect(source).toContain("import { get, set } from 'idb-keyval'");
  });

  it('exports replayMutations function', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export async function replayMutations');
    expect(source).toContain('get<MutationEntry[]>');
  });

  it('exports MutationEntry type', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export interface MutationEntry');
    expect(source).toContain('operation: string');
    expect(source).toContain('endpoint: string');
    expect(source).toContain('payload');
    expect(source).toContain('timestamp: number');
  });

  it('exports getQueueLength function', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export async function getQueueLength');
  });
});
```

## Change

### File: `remote-frontend/package.json` (EDIT — add idb-keyval)

- **Anchor:** `"dependencies"` block, the new `"workbox-window"` line from task 200
- **Before:**
  ```
  "workbox-window": "^8.0.0"
  ```
- **After:**
  ```
  "workbox-window": "^8.0.0",
  "idb-keyval": "^6.2.1"
  ```
  Run `cd remote-frontend && npm install`.

### File: `remote-frontend/src/lib/mutation-queue.ts` (CREATE)

```ts
import { get, set } from 'idb-keyval';

export interface MutationEntry {
  id: string;
  operation: string;
  endpoint: string;
  payload: unknown;
  timestamp: number;
}

const QUEUE_KEY = 'offline-mutation-queue';

export async function enqueueMutation(
  operation: string,
  endpoint: string,
  payload: unknown,
): Promise<void> {
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  const entry: MutationEntry = {
    id: crypto.randomUUID(),
    operation,
    endpoint,
    payload,
    timestamp: Date.now(),
  };
  const updated = queue ? [...queue, entry] : [entry];
  await set(QUEUE_KEY, updated);
}

export async function replayMutations(
  execute: (entry: MutationEntry) => Promise<void>,
  onError: (entry: MutationEntry, error: Error) => void,
): Promise<void> {
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  if (!queue || queue.length === 0) return;

  const remaining: MutationEntry[] = [];

  for (const entry of queue) {
    try {
      await execute(entry);
    } catch (err) {
      onError(entry, err instanceof Error ? err : new Error(String(err)));
      remaining.push(entry);
    }
  }

  await set(QUEUE_KEY, remaining);
}

export async function getQueueLength(): Promise<number> {
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  return queue?.length ?? 0;
}
```

### File: `remote-frontend/src/components/layout/Navbar.tsx` (EDIT — three anchors)

**Anchor A — top-level imports. Before the `const NAV_ITEMS` line:**

Before:
```tsx
import { Link, useLocation } from 'react-router-dom';
import { FolderOpen, ListTodo, LogOut } from 'lucide-react';
import { cn } from '@/lib/utils';
import { oauthApi } from '@/lib/api/oauth';
```

After:
```tsx
import { Link, useLocation } from 'react-router-dom';
import { useState, useEffect } from 'react';
import { FolderOpen, ListTodo, LogOut } from 'lucide-react';
import { cn } from '@/lib/utils';
import { oauthApi } from '@/lib/api/oauth';
import { getQueueLength } from '@/lib/mutation-queue';
```

**Anchor B — component body. After `const location = useLocation();`:**

Before (after task 203 applied):
```tsx
  const location = useLocation();

  const syncColor = {
```

After:
```tsx
  const location = useLocation();
  const [queueLength, setQueueLength] = useState(0);

  useEffect(() => {
    const update = () => {
      getQueueLength().then(setQueueLength).catch(() => {});
    };
    update();
    const interval = setInterval(update, 5_000);
    return () => clearInterval(interval);
  }, []);

  const syncColor = {
```

**Anchor C — link markup, add queue badge (after the sync dot span from task 203):**

- **Anchor:** the "VK Swarm" link with the sync dot (from task 203's edit)

Before (after task 203 applied):
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

After:
```tsx
            <Link to="/nodes" className="text-foreground font-semibold flex items-center gap-2">
              VK Swarm
              <span
                className={`inline-block w-2 h-2 rounded-full ${syncColor[syncStatus]}`}
                title={`Sync: ${syncStatus}`}
                aria-label={`Sync status: ${syncStatus}`}
              />
              {queueLength > 0 && (
                <span className="inline-flex items-center justify-center w-5 h-5 text-xs bg-amber-500 text-black rounded-full font-bold">
                  {queueLength}
                </span>
              )}
            </Link>
```

## Allowed moves

- Edit `remote-frontend/package.json`: add `"idb-keyval": "^6.2.1"` to dependencies. Run `npm install`.
- Create `remote-frontend/src/lib/mutation-queue.ts` with the exact code above.
- Create `remote-frontend/src/lib/mutation-queue.test.ts` with the exact code above.
- Edit `remote-frontend/src/components/layout/Navbar.tsx` at exactly three anchors (A: top-level imports, B: component body after useLocation, C: queue badge in link).
- Do NOT touch any other file.

## STOP triggers

- `idb-keyval` v6 doesn't resolve with the project's npm setup. If npm fails, try `idb-keyval@^6.2.1` explicitly — the `^6.2.1` range is correct. If the version is not available, use latest 6.x.
- Task 203 is not complete (Navbar.tsx doesn't have the sync dot markup yet). The Before anchors will not match. Verify with `git diff`.
- The Navbar.tsx test `NormalLayout.test.tsx` now expects `useEffect`/`getQueueLength` to not throw in jsdom. `idb-keyval` uses IndexedDB, which is available in jsdom. If jsdom's IndexedDB is not configured, mock `idb-keyval` in the test setup — record in decisions ledger.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/mutation-queue" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 204` exits 0.