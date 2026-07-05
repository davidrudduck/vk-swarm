import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider, UseQueryResult } from '@tanstack/react-query';
import { Nodes } from './Nodes';
import type { Organization } from '@/types/shared/types';

vi.mock('@/hooks/useOrganizations', () => ({
  useOrganizations: vi.fn(),
}));

vi.mock('@/lib/api', () => ({
  nodesApi: {
    list: vi.fn(),
  },
}));

vi.mock('@/components/swarm/NodeCard', () => ({
  NodeCard: ({ node }: { node: { name: string } }) => (
    <div data-testid="node-card">{node.name}</div>
  ),
}));

import { useOrganizations } from '@/hooks/useOrganizations';
import { nodesApi } from '@/lib/api';

describe('Nodes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  function renderNodes() {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });

    return render(
      <QueryClientProvider client={queryClient}>
        <Nodes />
      </QueryClientProvider>
    );
  }

  function createMockQuery<T>(
    data: T | undefined,
    isLoading: boolean,
    isError: boolean,
    error: Error | null
  ): UseQueryResult<T> {
    return {
      data,
      isLoading,
      isError,
      error,
      status: isLoading ? 'pending' : isError ? 'error' : 'success',
      isPending: isLoading,
      isPendingError: false,
      isSuccess: !isLoading && !isError,
      isRefetching: false,
      dataUpdatedAt: 0,
      errorUpdatedAt: 0,
      failureCount: 0,
      failureReason: null,
      refetch: vi.fn(),
    } as unknown as UseQueryResult<T>;
  }

  it('renders loading state initially', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>([], true, false, null)
    );

    renderNodes();

    // Should show the heading
    expect(screen.getByText('Nodes')).toBeInTheDocument();
  });

  it('renders "no nodes" message when no organization', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>([], false, false, null)
    );

    renderNodes();

    await waitFor(() => {
      expect(screen.getByText('Nodes are a swarm feature. Connect a hive server to get started.')).toBeInTheDocument();
    });
  });

  it('renders nodes when data is available', async () => {
    const mockOrganization = { id: 'org1', name: 'Test Org', slug: 'test-org', is_personal: false, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
    const mockNode = {
      id: 'n1',
      name: 'node-1',
      organization_id: 'org1',
      machine_id: 'm1',
      status: 'online' as const,
      capabilities: {
        executors: ['python'],
        max_concurrent_tasks: 4,
        os: 'linux',
        arch: 'x86_64',
        version: '1.0.0',
        git_commit: 'abc123',
        git_branch: 'main',
      },
      public_url: null,
      last_heartbeat_at: new Date().toISOString(),
      connected_at: new Date().toISOString(),
      disconnected_at: null,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    };

    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>([mockOrganization], false, false, null)
    );

    vi.mocked(nodesApi.list).mockResolvedValue([mockNode]);

    renderNodes();

    await waitFor(() => {
      expect(screen.getByText('node-1')).toBeInTheDocument();
      expect(screen.getByTestId('node-card')).toBeInTheDocument();
    });
  });

  it('renders "no nodes connected" when list is empty', async () => {
    const mockOrganization = { id: 'org1', name: 'Test Org', slug: 'test-org', is_personal: false, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };

    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>([mockOrganization], false, false, null)
    );

    vi.mocked(nodesApi.list).mockResolvedValue([]);

    renderNodes();

    await waitFor(() => {
      expect(screen.getByText('No nodes connected yet.')).toBeInTheDocument();
    });
  });

  it('renders error message on fetch failure', async () => {
    const mockOrganization = { id: 'org1', name: 'Test Org', slug: 'test-org', is_personal: false, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };

    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>(
        [mockOrganization],
        false,
        true,
        new Error('Fetch failed')
      )
    );

    vi.mocked(nodesApi.list).mockRejectedValue(new Error('Fetch failed'));

    renderNodes();

    await waitFor(() => {
      expect(screen.getByText('Failed to load nodes.')).toBeInTheDocument();
    });
  });
});
