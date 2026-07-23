import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { NodesPage } from './NodesPage';

// Mocks use the REAL hive `Node` shape (`@/types/nodes`, mirroring
// crates/remote/src/nodes/domain.rs): OS comes from `capabilities.os` (there is
// no `os_info`/`hostname` field), plus the required `machine_id` /
// `organization_id` fields. Earlier mocks used non-existent `os_info`/`hostname`,
// so `capabilities` was undefined and every node defaulted to 'linux' — the test
// could not have caught an OS-detection regression (adversarial review F7/F10).
const mockNodes = [
  {
    id: 'n1',
    organization_id: 'org-1',
    name: 'justX',
    machine_id: 'm1',
    status: 'online',
    capabilities: { os: 'mac', executors: [], max_concurrent_tasks: 1, arch: 'arm64', version: '1', git_commit: 'abc', git_branch: 'main' },
    public_url: 'u',
    last_heartbeat_at: '2026-07-04T10:00:00Z',
    connected_at: null,
    disconnected_at: null,
    created_at: '',
    updated_at: '',
  },
  {
    id: 'n2',
    organization_id: 'org-1',
    name: 'linux-01',
    machine_id: 'm2',
    status: 'online',
    capabilities: { os: 'linux', executors: [], max_concurrent_tasks: 1, arch: 'x86_64', version: '1', git_commit: 'abc', git_branch: 'main' },
    public_url: null,
    last_heartbeat_at: '2026-07-04T10:00:00Z',
    connected_at: null,
    disconnected_at: null,
    created_at: '',
    updated_at: '',
  },
];
const mockOrgs = { organizations: [{ id: 'org-1', name: 'Acme', slug: 'acme', is_personal: false, created_at: '', updated_at: '', user_role: 'owner' }] };

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: RequestInfo | URL) => {
    const u = typeof url === 'string' ? url : '';
    if (u.includes('/v1/nodes/api-keys')) return { ok: true, json: async () => [] } as Response;
    if (u.includes('/v1/organizations')) return { ok: true, json: async () => mockOrgs } as Response;
    if (u.includes('/v1/nodes')) return { ok: true, json: async () => mockNodes } as Response;
    return { ok: true, json: async () => ({}) } as Response;
  });
});

describe('NodesPage (SC8)', () => {
  it('fetches /v1/nodes and renders NodeCards', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={qc}><NodesPage /></QueryClientProvider>);
    await waitFor(() => {
      expect(screen.getByText('justX')).toBeTruthy();
      expect(screen.getByText('linux-01')).toBeTruthy();
    });
  });

  it('detects OS from capabilities.os (mac node renders a different glyph than linux)', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { container } = render(<QueryClientProvider client={qc}><NodesPage /></QueryClientProvider>);
    await waitFor(() => expect(screen.getByText('justX')).toBeTruthy());
    const glyphs = Array.from(container.querySelectorAll('.vks-node__os path')).map((p) => p.getAttribute('d'));
    // A broken mock (missing capabilities.os) would default BOTH nodes to 'linux',
    // yielding identical glyph paths. Distinct paths prove OS detection flows.
    expect(glyphs.length).toBeGreaterThanOrEqual(2);
    expect(glyphs[0]).not.toEqual(glyphs[1]);
  });

  it('renders a distinct error state (not an empty list) when the fetch fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(async () =>
      ({ ok: false, status: 500, text: async () => 'boom' }) as Response
    );
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={qc}><NodesPage /></QueryClientProvider>);
    await waitFor(() => expect(screen.getByTestId('page-error-banner')).toBeTruthy());
    expect(screen.getByText(/Failed to load nodes/i)).toBeTruthy();
  });
});
