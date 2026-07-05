---
id: "310"
phase: 3
title: App integration smoke + reachability gate evidence
status: ready
depends_on: ["307","308","309"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/app-integration.test.tsx
  - remote-frontend/src/index.css
  - docs/plans/vk-swarm-design-system/decisions-ledger.md
irreversible: false
scope_test: "remote-frontend/src/app-integration.test.tsx"
allowed_change: mixed
covers_criteria: [SC8, SC9]
---

## Sibling alignment

Read the full remote-frontend after tasks 307-309: AppRouter renders Chrome Navbar + BoardPage (BoardView+TaskDrawer) at `/tasks` + NodesPage (NodesView) at `/nodes` + ProcessesPage (ProcessesView) at `/processes`. The integration smoke test exercises the full provider tree (QueryClientProvider > ProfileProvider > Router) end-to-end with mocked fetch. Read `frontend/src/` — confirm it is NOT modified (SC9 parity). The reachability gate (per execute skill) requires: (a) CALL-PATH TRACE, (b) REAL-SEAM TEST, (c) INCIDENT-SYMPTOM ASSERTION. This task produces (b) the real-seam integration test + records (a)/(c) in the decisions-ledger.

## Failing test (write first)

Create `remote-frontend/src/app-integration.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ProfileProvider } from '@/components/ProfileProvider';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { createRoutes } from './AppRouter';

const mockTasks = [
  { id: 't1', title: 'Wire OAuth', status: 'inprogress', source_node_id: 'justX', labels: ['auth'], description: 'd', executing_node_id: 'justX' },
  { id: 't2', title: 'Add rate limit', status: 'todo', source_node_id: 'linux-01', labels: ['infra'] },
];
const mockNodes = [
  { id: 'n1', name: 'justX', os_info: 'mac', status: 'online', last_heartbeat_at: '2026-07-04T10:00:00Z', hostname: 'h', public_url: 'u' },
];

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: any) => {
    if (typeof url === 'string' && url.includes('/v1/profile')) return { ok: true, json: async () => ({ user_id: 'u1', username: 'david', email: 'd@e.io', providers: [] }) } as Response;
    if (typeof url === 'string' && url.includes('/v1/tasks/bulk')) return { ok: true, json: async () => ({ tasks: mockTasks, deleted_task_ids: [], latest_seq: 1 }) } as Response;
    if (typeof url === 'string' && url.includes('/v1/nodes')) return { ok: true, json: async () => mockNodes } as Response;
    return { ok: true, json: async () => [] } as Response;
  });
});

describe('app integration (SC8 real-seam)', () => {
  it('ProfileProvider > QueryClient > Router: /tasks renders Chrome Navbar + BoardView with fetched TaskCards + TaskDrawer opens', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: 0 } } });
    const router = createMemoryRouter(createRoutes(), { initialEntries: ['/tasks'] });
    render(
      <QueryClientProvider client={qc}>
        <ProfileProvider>
          <RouterProvider router={router} />
        </ProfileProvider>
      </QueryClientProvider>
    );
    await waitFor(() => expect(screen.getByText('Wire OAuth')).toBeTruthy());
    expect(screen.getByText('Board')).toBeTruthy(); // Chrome NavTab
    fireEvent.click(screen.getByText('Wire OAuth'));
    expect(screen.getByText('Merge')).toBeTruthy(); // TaskDrawer footer
  });

  it('/nodes renders Chrome Navbar + NodeCards', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: 0 } } });
    const router = createMemoryRouter(createRoutes(), { initialEntries: ['/nodes'] });
    render(
      <QueryClientProvider client={qc}>
        <ProfileProvider>
          <RouterProvider router={router} />
        </ProfileProvider>
      </QueryClientProvider>
    );
    await waitFor(() => expect(screen.getByText('justX')).toBeTruthy());
    expect(screen.getByText('Nodes')).toBeTruthy(); // Chrome NavTab
  });
});
```

## Change

### File: `remote-frontend/src/app-integration.test.tsx` (CREATE)
Create exactly as written in Failing test above.

### File: `remote-frontend/src/index.css` (EDIT — only if a `@import` is missing; otherwise no-op)
Verify the full `@import` chain is present: fonts → colors → typography → spacing → base → components. If task 208 already wired components.css, this is a no-op. If any `@import` is missing, add it.

## Reachability gate (orchestrator job — record in decisions-ledger, NOT in this task file)

After this task passes, the orchestrator MUST record in `docs/plans/vk-swarm-design-system/decisions-ledger.md` under `## Reachability gate`:
- **(a) CALL-PATH TRACE:** production entry point = `remote-frontend/src/main.tsx` → `<App />` → `<AppRouter />` → `createBrowserRouter(createRoutes())` → `/tasks` route → `<BoardPage />` → `useQuery(tasksApi.bulk)` → `fetch('/v1/tasks/bulk', { Authorization: Bearer <localStorage> })` → hive `crates/remote/src/routes/tasks.rs:36 bulk_shared_tasks` → `Json(bulk_tasks)` bare JSON → `groupByStatus` → `<BoardView columns={...} />` → `<TaskCard>` per row. Cite file:line for each hop.
- **(b) REAL-SEAM TEST:** `app-integration.test.tsx` (this task) — drives the real `ProfileProvider > QueryClientProvider > RouterProvider > BoardPage` seam with mocked fetch (the network boundary), NOT a mock past the changed unit.
- **(c) INCIDENT-SYMPTOM ASSERTION:** the symptom was "no cross-node task board" (spec Intent §1) → the test asserts `screen.getByText('Wire OAuth')` (a fetched task renders in the board), which would be absent if the wiring was dead.

## Allowed moves

- Create `app-integration.test.tsx`.
- Edit `index.css` only if an `@import` is missing (no-op otherwise).
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `createRoutes` not exported from `AppRouter.tsx` (task 307/308/309 drift → STOP).
- The integration test cannot drive the full provider tree (would mean a provider is missing → STOP, escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/app-integration.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 310` exits 0 AND the decisions-ledger has a non-empty `## Reachability gate` section (verified by `bash ~/.claude/wai/scripts/wai-evidence.sh vk-swarm-design-system`).