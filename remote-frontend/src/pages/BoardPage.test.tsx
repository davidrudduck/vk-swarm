import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BoardPage } from './BoardPage';

// NOTE: `tasksApi.bulk` returns `BulkSharedTasksResponse.tasks` as `{ task, user }[]`
// (`TaskActivity[]`) per the real hive response shape
// (`crates/remote/src/routes/tasks.rs:50-64,654-659`), not a flat `Task[]` as an
// earlier plan draft assumed. Mocks below use the real wrapped shape (task 306/308
// ledger). The real `Task` interface (`src/lib/api/tasks.ts`) also has no
// `source_node_id` / `labels` fields — `owner_name`/`executing_node_id` stand in for
// the board's `node` field, and `labels` is always `[]` (field-gap ledgered in 308).
const mockTasks = [
  {
    task: {
      id: 't1',
      organization_id: 'org-1',
      project_id: null,
      swarm_project_id: 'proj-1',
      creator_user_id: null,
      assignee_user_id: null,
      executing_node_id: 'n1',
      owner_node_id: null,
      owner_name: 'n1',
      title: 'First',
      description: null,
      status: 'todo',
      version: 1,
      deleted_at: null,
      shared_at: null,
      archived_at: null,
      created_at: '',
      updated_at: '',
    },
    user: null,
  },
  {
    task: {
      id: 't2',
      organization_id: 'org-1',
      project_id: null,
      swarm_project_id: 'proj-1',
      creator_user_id: null,
      assignee_user_id: null,
      executing_node_id: 'n2',
      owner_node_id: null,
      owner_name: 'n2',
      title: 'Second',
      description: null,
      status: 'inprogress',
      version: 1,
      deleted_at: null,
      shared_at: null,
      archived_at: null,
      created_at: '',
      updated_at: '',
    },
    user: null,
  },
];
const mockOrgs = {
  organizations: [
    { id: 'org-1', name: 'Acme', slug: 'acme', is_personal: false, created_at: '', updated_at: '', user_role: 'owner' },
  ],
};
const mockProjects = { projects: [{ id: 'proj-1', name: 'Main', organization_id: 'org-1', nodes: [] }] };

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: RequestInfo | URL) => {
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
