---
id: "308"
phase: 3
title: Wire BoardView + TaskDrawer to data (REST primary, Electric enhancement)
status: ready
depends_on: ["301","304","305","306","307"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pages/BoardPage.tsx
  - remote-frontend/src/pages/BoardPage.test.tsx
  - remote-frontend/src/AppRouter.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/BoardPage.test.tsx"
allowed_change: mixed
covers_criteria: [SC8]
---

## Sibling alignment

Read `remote-frontend/src/AppRouter.tsx` (task 307, post-Chrome-integration). The `/tasks` route currently renders a placeholder div. Read `design-source/ui_kits/vk-swarm-app/board.jsx` (task 301 BoardView) + `panels.jsx` (task 304 TaskDrawer). Read `remote-frontend/src/lib/api/{tasks,organizations,swarmProjects}.ts` (task 305). The wiring: `BoardPage` first fetches organizations via `organizationsApi.list()`, picks the first org, fetches projects via `swarmProjectsApi.list(orgId)`, picks the first project, then fetches tasks via `tasksApi.bulk(projectId)` (REST, primary — Electric collections are enhancement-only per the task 306 known-gap ledger entry). `tasksApi.bulk` returns `BulkSharedTasksResponse { tasks, deleted_task_ids, latest_seq }` — unpack `.tasks` (per `crates/remote/src/routes/tasks.rs:50-64,654-659`). Group tasks by status into the `columns` shape BoardView expects, manage `selectedTask` state, and render `<BoardView columns={...} onAdd={...} onOpen={(t, status) => setSelectedTask(t)} selectedId={selectedTask?.id} />` + `<TaskDrawer task={selectedTask} status={selectedTaskStatus} onClose={() => setSelectedTask(null)} ... />`. Use `@tanstack/react-query` `useQuery` for each fetch (QueryClient is already wired in App.tsx from task 106). Record the chained-fetch + "first org/project" selection + REST-primary decision in the ledger.

## Failing test (write first)

Create `remote-frontend/src/pages/BoardPage.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BoardPage } from './BoardPage';

const mockTasks = [
  { id: 't1', title: 'First', status: 'todo', source_node_id: 'n1', labels: ['a'] },
  { id: 't2', title: 'Second', status: 'inprogress', source_node_id: 'n2', labels: [], executing_node_id: 'n2' },
];
const mockOrgs = { organizations: [{ id: 'org-1', name: 'Acme', slug: 'acme', is_personal: false, created_at: '', updated_at: '', user_role: 'owner' }] };
const mockProjects = { projects: [{ id: 'proj-1', name: 'Main', organization_id: 'org-1', nodes: [] }] };

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: any) => {
    const u = typeof url === 'string' ? url : '';
    if (u.includes('/v1/organizations')) return { ok: true, json: async () => mockOrgs } as Response;
    if (u.includes('/v1/swarm/projects')) return { ok: true, json: async () => mockProjects } as Response;
    if (u.includes('/v1/tasks/bulk')) return { ok: true, json: async () => ({ tasks: mockTasks, deleted_task_ids: [], latest_seq: 1 }) } as Response;
    return { ok: true, json: async () => ({}) } as Response;
  });
});

describe('BoardPage (SC8)', () => {
  it('fetches /v1/tasks/bulk and renders TaskCards grouped by status', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={qc}><BoardPage /></QueryClientProvider>);
    await waitFor(() => {
      expect(screen.getByText('First')).toBeTruthy();
      expect(screen.getByText('Second')).toBeTruthy();
    });
  });

  it('opens TaskDrawer when a TaskCard is clicked', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={qc}><BoardPage /></QueryClientProvider>);
    await waitFor(() => expect(screen.getByText('First')).toBeTruthy());
    fireEvent.click(screen.getByText('First'));
    expect(screen.getByText('Merge')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/pages/BoardPage.tsx` (CREATE)
```tsx
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { BoardView, COLUMNS, type TaskRow } from '@/ui/board';
import { TaskDrawer } from '@/ui/panels';
import { tasksApi, type Task } from '@/lib/api/tasks';
import { organizationsApi } from '@/lib/api/organizations';
import { swarmProjectsApi } from '@/lib/api/swarmProjects';
import type { TaskStatus } from '@/components/board';

export function BoardPage() {
  const [selected, setSelected] = useState<{ task: TaskRow; status: TaskStatus } | null>(null);
  const orgsQ = useQuery({ queryKey: ['orgs'], queryFn: organizationsApi.list });
  const orgId = orgsQ.data?.organizations[0]?.id;
  const projectsQ = useQuery({ queryKey: ['projects', orgId], queryFn: () => swarmProjectsApi.list(orgId!), enabled: !!orgId });
  const projectId = projectsQ.data?.projects[0]?.id;
  const tasksQ = useQuery({ queryKey: ['tasks', 'bulk', projectId], queryFn: () => tasksApi.bulk(projectId!), enabled: !!projectId });
  const tasks = tasksQ.data?.tasks ?? [];
  const columns = groupByStatus(tasks);
  return (
    <>
      <BoardView columns={columns} onAdd={() => {}} onOpen={(task, status) => setSelected({ task, status })} selectedId={selected?.task.id} />
      <TaskDrawer task={selected?.task ?? null} status={selected?.status ?? 'todo'} onClose={() => setSelected(null)} />
    </>
  );
}

function groupByStatus(tasks: Task[]): Record<TaskStatus, TaskRow[]> {
  const out: Record<TaskStatus, TaskRow[]> = { todo: [], inprogress: [], inreview: [], done: [], cancelled: [] };
  for (const t of tasks) {
    const s = (t.status ?? 'todo') as TaskStatus;
    if (s in out) out[s].push({ id: t.id, title: t.title, description: t.description, node: t.source_node_id, labels: t.labels, days: t.days, attempt: t.attempt });
  }
  return out;
}
```
The exact `Task` interface shape depends on what task 305 exports; the implementer should align `groupByStatus` to the actual `Task` fields. Record field-mapping decisions + the chained-fetch + first-org/project selection in the ledger.

### File: `remote-frontend/src/pages/BoardPage.test.tsx` (CREATE)
Create exactly as written in Failing test above.

### File: `remote-frontend/src/AppRouter.tsx` (EDIT)
Replace the `/tasks` placeholder with `import { BoardPage } from './pages/BoardPage'` + `element: <BoardPage />` (inside the Chrome layout).

## Allowed moves

- Create `BoardPage.tsx` + `BoardPage.test.tsx`.
- Edit `AppRouter.tsx` to swap the `/tasks` placeholder for `<BoardPage />`.
- Use `useQuery` from `@tanstack/react-query` (installed in task 100).
- Map `Task` fields to `TaskRow` fields in `groupByStatus` — record the exact mapping in the ledger.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `BoardView` not exported from `@/ui/board` (task 301 drift → STOP).
- `TaskDrawer` not exported from `@/ui/panels` (task 304 drift → STOP).
- `tasksApi.bulk` not exported from `@/lib/api/tasks` (task 305 drift → STOP).
- The `Task` interface from task 305 lacks `status` / `source_node_id` / `labels` fields needed for `groupByStatus` → STOP, record the field-gap in the ledger, and extend the mapping.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/BoardPage.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 308` exits 0.