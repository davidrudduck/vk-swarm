import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TooltipProvider } from '@/components/ui/tooltip';
import { NodeApiKeySection } from './NodeApiKeySection';
import { nodesApi } from '@/lib/api';
import type { NodeApiKey } from '@/types/nodes';
import enSettings from '../../../../frontend/src/i18n/locales/en/settings.json';

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
      return fallback || key;
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
      expect(screen.getByText('No API keys found. Create one to allow nodes to connect.')).toBeInTheDocument();
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
    expect(screen.getByText('Bound')).toBeInTheDocument();
    expect(screen.getByText('Unbound')).toBeInTheDocument();
    expect(screen.getByText(/Created 2026-01-01/)).toBeInTheDocument();
    expect(screen.getByText(/Last used 2026-01-03/)).toBeInTheDocument();
    expect(screen.queryByText(/Last used 2026-01-02/)).not.toBeInTheDocument();
  });

  it('opens the create Dialog, reveals the one-time secret, and supports show/hide + copy (TS4)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockResolvedValue({
      api_key: {
        id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      secret: 'vk_SECRET_VALUE_DO_NOT_SHARE',
    });

    const execCommand = vi.fn(() => true);
    document.execCommand = execCommand;
    // @ts-expect-error — assigning undefined to disable clipboard in test
    navigator.clipboard = undefined;

    const { fireEvent } = await import('@testing-library/react');
    renderWith(<NodeApiKeySection organizationId="org-1" />);

    fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    fireEvent.change(nameInput, { target: { value: 'Test Key' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(nodesApi.createApiKey).toHaveBeenCalledWith({ organization_id: 'org-1', name: 'Test Key' });
    });

    await waitFor(() => {
      expect(screen.getByText('••••••••••••••••••••')).toBeInTheDocument();
    });
    const secretWrapper = screen.getByText('••••••••••••••••••••').closest('[data-secret-wrapper]')!;
    expect(secretWrapper).toHaveAttribute('data-hidden', 'true');

    fireEvent.click(screen.getByRole('button', { name: 'Reveal' }));
    expect(secretWrapper).toHaveAttribute('data-hidden', 'false');
    expect(screen.getByText('vk_SECRET_VALUE_DO_NOT_SHARE')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Hide' }));
    expect(secretWrapper).toHaveAttribute('data-hidden', 'true');
    expect(screen.queryByText('vk_SECRET_VALUE_DO_NOT_SHARE')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
    expect(execCommand).toHaveBeenCalledWith('copy');

    fireEvent.click(screen.getByRole('button', { name: 'Done' }));
    await waitFor(() => {
      expect(screen.queryByText('vk_SECRET_VALUE_DO_NOT_SHARE')).not.toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await waitFor(() => {
      expect(screen.getByLabelText('Key Name')).toBeInTheDocument();
      expect(screen.queryByText('vk_SECRET_VALUE_DO_NOT_SHARE')).not.toBeInTheDocument();
    });
  });

  it('revokes a key only after window.confirm; query is invalidated on success (TS5)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const invalidateSpy = vi.fn();
    const localQueryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    localQueryClient.invalidateQueries = invalidateSpy;
    const keys: NodeApiKey[] = [{
      id: 'k1', organization_id: 'org-1', name: 'MacBook', key_prefix: 'vk_abc',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: 'n1', takeover_count: 0, takeover_window_start: null,
      blocked_at: null, blocked_reason: null,
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    vi.mocked(nodesApi.revokeApiKey).mockResolvedValue();

    render(
      <QueryClientProvider client={localQueryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-1" /></TooltipProvider>
      </QueryClientProvider>
    );
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    const revokeBtn = await screen.findByRole('button', { name: 'Revoke' });
    await u.click(revokeBtn);
    expect(confirmSpy).toHaveBeenCalled();
    await waitFor(() => {
      expect(nodesApi.revokeApiKey).toHaveBeenCalledWith('k1');
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['nodeApiKeys', 'org-1'] });
    });

    confirmSpy.mockReturnValue(false);
    vi.mocked(nodesApi.revokeApiKey).mockClear();
    await u.click(revokeBtn);
    expect(nodesApi.revokeApiKey).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('renders Blocked badge with reason; Unblock calls confirm + unblockApiKey + invalidates (TS6)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const invalidateSpy = vi.fn();
    const localQueryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    localQueryClient.invalidateQueries = invalidateSpy;
    const keys: NodeApiKey[] = [{
      id: 'k2', organization_id: 'org-1', name: 'Compromised', key_prefix: 'vk_xyz',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: 'n1', takeover_count: 5, takeover_window_start: '2026-01-01T00:00:00Z',
      blocked_at: '2026-01-02T00:00:00Z', blocked_reason: 'Duplicate key use detected',
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    vi.mocked(nodesApi.unblockApiKey).mockResolvedValue(keys[0]);

    render(
      <QueryClientProvider client={localQueryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-1" /></TooltipProvider>
      </QueryClientProvider>
    );
    expect(await screen.findByText('Blocked')).toBeInTheDocument();
    expect(screen.getByText('Duplicate key use detected')).toBeInTheDocument();
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Unblock' }));
    await waitFor(() => {
      expect(nodesApi.unblockApiKey).toHaveBeenCalledWith('k2');
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['nodeApiKeys', 'org-1'] });
    });
    confirmSpy.mockRestore();
  });

  it('surfaces a destructive Alert when a mutation rejects; the list does not refetch (TS7)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockRejectedValue(new Error('boom'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    const listSpy = vi.mocked(nodesApi.listApiKeys);
    const callsBefore = listSpy.mock.calls.length;
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    await u.type(nameInput, 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/boom/)).toBeInTheDocument();
    });
    expect(listSpy.mock.calls.length).toBe(callsBefore);
  });

  it('every settings.swarm.apiKeys.* key used in NodeApiKeySection.tsx has a matching key in the en locale (TS9)', () => {
    const requiredKeys = [
      'settings.swarm.apiKeys.title',
      'settings.swarm.apiKeys.description',
      'settings.swarm.apiKeys.create',
      'settings.swarm.apiKeys.createTitle',
      'settings.swarm.apiKeys.secretTitle',
      'settings.swarm.apiKeys.secretDescription',
      'settings.swarm.apiKeys.copyToClipboard',
      'settings.swarm.apiKeys.copied',
      'settings.swarm.apiKeys.showSecret',
      'settings.swarm.apiKeys.hideSecret',
      'settings.swarm.apiKeys.nameLabel',
      'settings.swarm.apiKeys.namePlaceholder',
      'settings.swarm.apiKeys.cancel',
      'settings.swarm.apiKeys.done',
      'settings.swarm.apiKeys.createAction',
      'settings.swarm.apiKeys.loading',
      'settings.swarm.apiKeys.empty',
      'settings.swarm.apiKeys.bound',
      'settings.swarm.apiKeys.unbound',
      'settings.swarm.apiKeys.created',
      'settings.swarm.apiKeys.lastUsed',
      'settings.swarm.apiKeys.revoked',
      'settings.swarm.apiKeys.blocked',
      'settings.swarm.apiKeys.revoke',
      'settings.swarm.apiKeys.revokeConfirm',
      'settings.swarm.apiKeys.unblock',
      'settings.swarm.apiKeys.unblockConfirm',
      'settings.swarm.apiKeys.error',
    ];
    const apiKeysBlock = (enSettings as any).settings?.swarm?.apiKeys ?? {};
    for (const key of requiredKeys) {
      const suffix = key.replace('settings.swarm.apiKeys.', '');
      expect(apiKeysBlock[suffix], `Missing i18n key: ${key}`).toBeDefined();
    }
  });
});
