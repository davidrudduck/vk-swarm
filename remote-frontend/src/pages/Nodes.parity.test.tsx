import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, UseQueryResult } from '@tanstack/react-query'
import { Nodes } from './Nodes'
import type { Node } from '@/types/nodes'
import type { Organization } from '@/types/shared/types'

// Mock the hooks
vi.mock('@/hooks/useOrganizations', () => ({
  useOrganizations: vi.fn(),
}))

vi.mock('@/lib/api', () => ({
  nodesApi: {
    list: vi.fn(),
  },
}))

vi.mock('@/components/swarm/NodeCard', () => ({
  NodeCard: ({ node }: { node: Node }) => (
    <div data-testid="node-card">{node.name}</div>
  ),
}))

import { useOrganizations } from '@/hooks/useOrganizations'
import { nodesApi } from '@/lib/api'

describe('Nodes.parity — hive Nodes page structural parity', () => {
  const mockOrganizations: Organization[] = [
    {
      id: 'org-1',
      name: 'Test Org',
      slug: 'test-org',
      is_personal: false,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
  ]

  const mockNodes: Node[] = [
    {
      id: 'node-1',
      organization_id: 'org-1',
      name: 'node-alpha',
      machine_id: 'machine-1',
      status: 'online',
      capabilities: {
        executors: ['droid', 'codex'],
        max_concurrent_tasks: 4,
        os: 'linux',
        arch: 'x86_64',
        version: '1.0.0',
        git_commit: 'abc123',
        git_branch: 'main',
      },
      public_url: null,
      last_heartbeat_at: '2024-01-01T00:00:00Z',
      connected_at: '2024-01-01T00:00:00Z',
      disconnected_at: null,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
    {
      id: 'node-2',
      organization_id: 'org-1',
      name: 'node-beta',
      machine_id: 'machine-2',
      status: 'offline',
      capabilities: {
        executors: ['droid'],
        max_concurrent_tasks: 2,
        os: 'macos',
        arch: 'arm64',
        version: '1.0.0',
        git_commit: 'def456',
        git_branch: 'main',
      },
      public_url: null,
      last_heartbeat_at: '2024-01-01T00:00:00Z',
      connected_at: '2024-01-01T00:00:00Z',
      disconnected_at: '2024-01-01T12:00:00Z',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
  ]

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
    } as unknown as UseQueryResult<T>
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  function renderNodesPage() {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    })

    return render(
      <QueryClientProvider client={queryClient}>
        <Nodes />
      </QueryClientProvider>
    )
  }

  it('renders "Nodes" heading', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>(mockOrganizations, false, false, null)
    )

    vi.mocked(nodesApi.list).mockResolvedValue(mockNodes)

    renderNodesPage()

    await waitFor(() => {
      const heading = screen.getByRole('heading', { level: 2, name: 'Nodes' })
      expect(heading).toBeInTheDocument()
    })
  })

  it('renders NodeCards with node data after loading', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>(mockOrganizations, false, false, null)
    )

    vi.mocked(nodesApi.list).mockResolvedValue(mockNodes)

    renderNodesPage()

    // Wait for nodes to render
    await waitFor(() => {
      expect(screen.getByText('node-alpha')).toBeInTheDocument()
      expect(screen.getByText('node-beta')).toBeInTheDocument()
    })

    // Assert we have 2 NodeCards rendered
    const nodeCards = screen.getAllByTestId('node-card')
    expect(nodeCards.length).toBe(2)
  })

  it('renders loading state when organizations are loading', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>([], true, false, null)
    )

    renderNodesPage()

    // The page should still render the heading while loading
    expect(screen.getByText('Nodes')).toBeInTheDocument()
  })

  it('renders empty state when no organizations', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>([], false, false, null)
    )

    renderNodesPage()

    await waitFor(() => {
      expect(
        screen.getByText(
          'Nodes are a swarm feature. Connect a hive server to get started.'
        )
      ).toBeInTheDocument()
    })
  })

  it('renders error state when nodes list fails', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>(mockOrganizations, false, true, new Error('API Error'))
    )

    vi.mocked(nodesApi.list).mockRejectedValue(new Error('API Error'))

    renderNodesPage()

    await waitFor(() => {
      expect(screen.getByText('Failed to load nodes.')).toBeInTheDocument()
    })
  })

  it('renders empty nodes state when list is empty', async () => {
    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>(mockOrganizations, false, false, null)
    )

    vi.mocked(nodesApi.list).mockResolvedValue([])

    renderNodesPage()

    await waitFor(() => {
      expect(screen.getByText('No nodes connected yet.')).toBeInTheDocument()
    })
  })

  it('structural parity: same rendered elements as frontend reference', async () => {
    // This test verifies that the hive Nodes page (remote-frontend) renders
    // the same core structural elements as the node Nodes page (frontend).
    //
    // Both pages should have:
    // 1. An h2 "Nodes" heading
    // 2. NodeCards rendered in a grid when data is present
    // 3. Identical loading/error/empty states
    //
    // The rendering order and component structure should match.

    vi.mocked(useOrganizations).mockReturnValue(
      createMockQuery<Organization[]>(mockOrganizations, false, false, null)
    )

    vi.mocked(nodesApi.list).mockResolvedValue(mockNodes)

    const { container } = renderNodesPage()

    await waitFor(() => {
      // Verify heading exists
      const heading = container.querySelector('h2')
      expect(heading?.textContent).toBe('Nodes')

      // Verify NodeCards render with node names
      const alpha = screen.getByText('node-alpha')
      const beta = screen.getByText('node-beta')
      expect(alpha).toBeTruthy()
      expect(beta).toBeTruthy()

      // Verify grid container exists
      const grid = container.querySelector('[class*="grid"]')
      expect(grid).toBeTruthy()

      // Verify node cards are rendered
      const cards = screen.getAllByTestId('node-card')
      expect(cards.length).toBe(2)
    })
  })
})
