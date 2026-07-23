import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { NodesPage } from './NodesPage';

const mockNodes = [
  { id: 'n1', name: 'justX', os_info: 'mac', status: 'online', last_heartbeat_at: '2026-07-04T10:00:00Z', hostname: 'h', public_url: 'u' },
  { id: 'n2', name: 'linux-01', os_info: 'linux', status: 'online', last_heartbeat_at: '2026-07-04T10:00:00Z', hostname: null, public_url: null },
];
const mockOrgs = { organizations: [{ id: 'org-1', name: 'Acme', slug: 'acme', is_personal: false, created_at: '', updated_at: '', user_role: 'owner' }] };

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: RequestInfo | URL) => {
    const u = typeof url === 'string' ? url : '';
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
});
