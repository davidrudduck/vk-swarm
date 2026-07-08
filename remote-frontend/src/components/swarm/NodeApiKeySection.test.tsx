import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TooltipProvider } from '@/components/ui/tooltip';
import { NodeApiKeySection } from './NodeApiKeySection';
import { nodesApi } from '@/lib/api';
import type { NodeApiKey } from '@/types/nodes';

vi.mock('@/lib/api', () => ({
  nodesApi: {
    listApiKeys: vi.fn(),
    createApiKey: vi.fn(),
    revokeApiKey: vi.fn(),
    unblockApiKey: vi.fn(),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string, options?: Record<string, unknown>) => {
      if (options && typeof options === 'object') {
        return (fallback || key).replace(/\{\{(\w+)\}\}/g, (_, name) => String(options[name] ?? ''));
      }
      return key;
    },
    i18n: { language: 'en' },
  }),
}));

describe('NodeApiKeySection', () => {
  let queryClient: QueryClient;
  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    vi.clearAllMocks();
  });
  const renderWith = (ui: React.ReactElement) =>
    render(
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>{ui}</TooltipProvider>
      </QueryClientProvider>
    );

  it('renders without throwing when organizationId is set and query is loading (TS1)', () => {
    vi.mocked(nodesApi.listApiKeys).mockReturnValue(new Promise(() => {}));
    expect(() => renderWith(<NodeApiKeySection organizationId="org-1" />)).not.toThrow();
  });

  it('renders the empty-state copy when the query returns [] (TS2)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await waitFor(() => {
      expect(screen.getByText('settings.swarm.apiKeys.empty')).toBeInTheDocument();
    });
  });

  it('renders one ApiKeyItem per active key with name, key_prefix, bound/unbound badge, created + last-used timestamps (TS3)', async () => {
    const keys: NodeApiKey[] = [
      {
        id: 'k1', organization_id: 'org-1', name: 'MacBook', key_prefix: 'vk_abc',
        created_by: null, last_used_at: '2026-01-03T00:00:00Z', revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: 'n1', takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      {
        id: 'k2', organization_id: 'org-1', name: 'Build', key_prefix: 'vk_xyz',
        created_by: null, last_used_at: null, revoked_at: null,
        created_at: '2026-01-02T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
    ];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await waitFor(() => {
      expect(screen.getByText('MacBook')).toBeInTheDocument();
      expect(screen.getByText('Build')).toBeInTheDocument();
      expect(screen.getByText('vk_abc')).toBeInTheDocument();
      expect(screen.getByText('vk_xyz')).toBeInTheDocument();
    });
    expect(screen.getByText('settings.swarm.apiKeys.bound')).toBeInTheDocument();
    expect(screen.getByText('settings.swarm.apiKeys.unbound')).toBeInTheDocument();
    expect(screen.getByText(/Created 2026-01-01/)).toBeInTheDocument();
    expect(screen.getByText(/Last used 2026-01-03/)).toBeInTheDocument();
    expect(screen.queryByText(/Last used 2026-01-02/)).not.toBeInTheDocument();
  });
});
