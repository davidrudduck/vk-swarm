---
id: "305"
phase: 3
title: Create cross-node TasksBoard page consuming the 3 new Electric collections
status: done
depends_on: ["304"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pages/Tasks.tsx
  - remote-frontend/src/pages/Tasks.test.tsx
  - remote-frontend/src/AppRouter.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Tasks.test.tsx"
allowed_change: mixed
covers_criteria: [SC2, SC5]
---
## Failing test (write first)

Create `remote-frontend/src/pages/Tasks.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

// Mock @tanstack/react-db so the collection hooks return predictable rows
vi.mock('@tanstack/react-db', () => ({
  useCollection: vi.fn((collection) => ({
    data: collection._mockRows ?? [],
    isLoading: false,
  })),
}));

// Mock the 3 collections to return tagged row sets
vi.mock('@/lib/electric', () => ({
  createTaskAssignmentsCollection: () => ({ _mockRows: [
    { id: 'a1', task_id: 't1', node_id: 'n1', execution_status: 'in_progress', assigned_at: '2026-07-04T00:00:00Z', started_at: '2026-07-04T00:00:00Z', completed_at: null, lease_expires_at: null, fencing_token: 1, local_task_id: null, local_attempt_id: null, node_project_id: 'np1', created_at: '2026-07-04T00:00:00Z' },
    { id: 'a2', task_id: 't2', node_id: 'n2', execution_status: 'pending', assigned_at: '2026-07-04T00:00:00Z', started_at: null, completed_at: null, lease_expires_at: null, fencing_token: 0, local_task_id: null, local_attempt_id: null, node_project_id: 'np2', created_at: '2026-07-04T00:00:00Z' },
  ]}),
  createTaskOutputLogsCollection: () => ({ _mockRows: [] }),
  createTaskProgressEventsCollection: () => ({ _mockRows: [] }),
  createNodesCollection: () => ({ _mockRows: [
    { id: 'n1', name: 'node-alpha', organization_id: 'org1', hostname: null, os_info: null, status: 'online', last_heartbeat_at: null, public_url: null, created_at: '', updated_at: '' },
    { id: 'n2', name: 'node-beta', organization_id: 'org1', hostname: null, os_info: null, status: 'online', last_heartbeat_at: null, public_url: null, created_at: '', updated_at: '' },
  ]}),
  createProjectsCollection: () => ({ _mockRows: [] }),
}));

import { TasksBoard } from './Tasks';

describe('TasksBoard', () => {
  it('renders tasks grouped by execution_status across multiple nodes', () => {
    render(<TasksBoard />);
    // Pending column
    expect(screen.getByText(/pending/i)).toBeInTheDocument();
    // In-progress column
    expect(screen.getByText(/in_progress|in progress/i)).toBeInTheDocument();
    // Cross-node: assignments come from n1 and n2
    expect(screen.getByText('node-alpha')).toBeInTheDocument();
    expect(screen.getByText('node-beta')).toBeInTheDocument();
  });
});
```

Test fails red — `TasksBoard` not yet defined.

## Change

### File: `remote-frontend/src/pages/Tasks.tsx` (CREATE)

**Sibling alignment:** Read `frontend/src/pages/Nodes.tsx` (the swarm management page). It uses `@tanstack/react-query` `useQuery` with REST API clients — NOT Electric collections. The `TasksBoard` is a NET-NEW page that uses `@tanstack/react-db` `useCollection` with the Electric collections (a different data layer than the rehosted swarm components). Justify the divergence in the decisions ledger: the spec's Design section (track 3) says reads via Electric collections, writes via REST — this page is the Electric-reads half.

```tsx
import { useCollection } from '@tanstack/react-db';
import {
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  createNodesCollection,
  createProjectsCollection,
} from '@/lib/electric';

const assignmentsCollection = createTaskAssignmentsCollection();
const outputLogsCollection = createTaskOutputLogsCollection();
const progressEventsCollection = createTaskProgressEventsCollection();
const nodesCollection = createNodesCollection();
const projectsCollection = createProjectsCollection();

const STATUS_COLUMNS = ['pending', 'in_progress', 'completed', 'failed'] as const;

export function TasksBoard() {
  const { data: assignments } = useCollection(assignmentsCollection);
  const { data: nodes } = useCollection(nodesCollection);
  const { data: projects } = useCollection(projectsCollection);

  const nodeNames = new Map(nodes.map((n) => [n.id, n.name]));
  const projectNames = new Map(projects.map((p) => [p.id, p.name]));

  const byStatus = new Map<string, typeof assignments>();
  for (const status of STATUS_COLUMNS) byStatus.set(status, []);
  for (const a of assignments) {
    const bucket = byStatus.get(a.execution_status) ?? byStatus.get('pending');
    bucket?.push(a);
  }

  return (
    <div className="flex gap-4">
      {STATUS_COLUMNS.map((status) => (
        <div key={status} className="flex-1">
          <h2 className="text-lg font-semibold capitalize">{status.replace('_', ' ')}</h2>
          <ul>
            {(byStatus.get(status) ?? []).map((a) => (
              <li key={a.id} className="border p-2 my-2">
                <div>task {a.task_id}</div>
                <div>{nodeNames.get(a.node_id) ?? a.node_id}</div>
                <div>{projectNames.get(a.node_project_id) ?? a.node_project_id}</div>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

export default TasksBoard;
```

### File: `remote-frontend/src/AppRouter.tsx`

Replace the `/tasks` placeholder route (set in 105) with the real component:

**Before:**
```tsx
{ path: '/tasks', element: <div>Tasks placeholder</div> }
```

**After:**
```tsx
import { TasksBoard } from './pages/Tasks';
// ...
{ path: '/tasks', element: <TasksBoard /> }
```

## Allowed moves
- CREATE `remote-frontend/src/pages/Tasks.tsx` with `TasksBoard` (Electric-collection-backed kanban).
- CREATE `remote-frontend/src/pages/Tasks.test.tsx`.
- EDIT `remote-frontend/src/AppRouter.tsx` to swap the `/tasks` placeholder for `<TasksBoard />`.
- No server changes. No new push channels (Electric shape polling only).

## STOP triggers
- `@/lib/electric` does not resolve from `remote-frontend/` — HALT; task 201's `@/*` alias is missing or 304's exports are absent.
- `useCollection` is not exported by `@tanstack/react-db` in the installed version — HALT; record in ledger and pick the documented API surface for the installed version (check `remote-frontend/node_modules/@tanstack/react-db/dist/` exports).

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/pages/Tasks.test.tsx
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
```
All exit 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Tasks.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 305` exits 0