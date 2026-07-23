import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ProfileProvider } from '@/components/ProfileProvider';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { createRoutes } from './AppRouter';

// Real-seam integration test (SC8): drives the full production provider tree
// `QueryClientProvider > ProfileProvider > RouterProvider(createRoutes())` with
// `fetch` mocked at the network boundary (not past the changed units).
//
// Mock payloads use the REAL, already-shipped hive contracts rather than the
// plan-literal draft shapes (plan-drift precedent, tasks 306/308/309 ledger):
//   - `/v1/organizations` -> `{ organizations: [...] }` (unwrapped by the api client)
//   - `/v1/swarm/projects` -> `{ projects: [...] }`
//   - `/v1/tasks/bulk` -> `{ tasks: TaskActivity[] }` where each is `{ task, user }`;
//     the real `Task` has no `source_node_id`/`labels` fields (`node` derives from
//     `owner_name`).
//   - `/v1/nodes` -> `Node[]` with `capabilities.os` (no `os_info`/`hostname` fields).
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
      owner_name: 'justX',
      title: 'Wire OAuth',
      description: 'd',
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
  {
    task: {
      id: 't2',
      organization_id: 'org-1',
      project_id: null,
      swarm_project_id: 'proj-1',
      creator_user_id: null,
      assignee_user_id: null,
      executing_node_id: 'linux-01',
      owner_node_id: null,
      owner_name: 'linux-01',
      title: 'Add rate limit',
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
];
const mockOrgs = {
  organizations: [
    { id: 'org-1', name: 'Acme', slug: 'acme', is_personal: false, created_at: '', updated_at: '', user_role: 'owner' },
  ],
};
const mockProjects = { projects: [{ id: 'proj-1', name: 'Main', organization_id: 'org-1', nodes: [] }] };
const mockNodes = [
  {
    id: 'n1',
    organization_id: 'org-1',
    name: 'justX',
    machine_id: 'm1',
    status: 'online',
    capabilities: { os: 'mac' },
    public_url: 'u',
    last_heartbeat_at: '2026-07-04T10:00:00Z',
  },
];

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: RequestInfo | URL) => {
    const u = typeof url === 'string' ? url : url instanceof URL ? url.toString() : '';
    if (u.includes('/v1/profile')) return { ok: true, json: async () => ({ user_id: 'u1', username: 'david', email: 'd@e.io', providers: [] }) } as Response;
    if (u.includes('/v1/organizations')) return { ok: true, json: async () => mockOrgs } as Response;
    if (u.includes('/v1/swarm/projects')) return { ok: true, json: async () => mockProjects } as Response;
    if (u.includes('/v1/tasks/bulk')) return { ok: true, json: async () => ({ tasks: mockTasks, deleted_task_ids: [], latest_seq: 1 }) } as Response;
    if (u.includes('/v1/nodes/api-keys')) return { ok: true, json: async () => [] } as Response;
    if (u.includes('/v1/nodes')) return { ok: true, json: async () => mockNodes } as Response;
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
    // Longer timeout: this seam lazy-loads NodesPage and runs a chained
    // orgs -> nodes query before the NodeCard grid mounts.
    await waitFor(() => expect(screen.getByText('justX')).toBeTruthy(), { timeout: 5000 });
    expect(screen.getByText('Nodes')).toBeTruthy(); // Chrome NavTab
  });
});
