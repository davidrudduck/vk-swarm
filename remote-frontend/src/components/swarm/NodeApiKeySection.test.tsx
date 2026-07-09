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
    const origExecCommand = document.execCommand;
    document.execCommand = execCommand;
    const origClipboard = navigator.clipboard;
    // @ts-expect-error — assigning undefined to disable clipboard in test
    navigator.clipboard = undefined;

    try {
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
    } finally {
      Object.defineProperty(navigator, 'clipboard', { value: origClipboard, configurable: true });
      document.execCommand = origExecCommand;
    }
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

  it('revoke invalidation targets the org active when the mutation started, not the org at callback time (TS5b)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const invalidateSpy = vi.fn();
    const localQueryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    localQueryClient.invalidateQueries = invalidateSpy;
    const keysOrg1: NodeApiKey[] = [{
      id: 'k1', organization_id: 'org-1', name: 'MacBook', key_prefix: 'vk_abc',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: 'n1', takeover_count: 0, takeover_window_start: null,
      blocked_at: null, blocked_reason: null,
    }];
    vi.mocked(nodesApi.listApiKeys).mockImplementation((orgId: string) => Promise.resolve(orgId === 'org-1' ? keysOrg1 : []));
    let resolveRevoke: () => void = () => {};
    vi.mocked(nodesApi.revokeApiKey).mockImplementation(() => new Promise((resolve) => { resolveRevoke = resolve; }));

    const result = render(
      <QueryClientProvider client={localQueryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-1" /></TooltipProvider>
      </QueryClientProvider>
    );
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    const revokeBtn = await screen.findByRole('button', { name: 'Revoke' });
    await u.click(revokeBtn);
    expect(nodesApi.revokeApiKey).toHaveBeenCalled();

    result.rerender(
      <QueryClientProvider client={localQueryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-2" /></TooltipProvider>
      </QueryClientProvider>
    );
    await waitFor(() => expect(nodesApi.listApiKeys).toHaveBeenCalledWith('org-2'));

    resolveRevoke();
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['nodeApiKeys', 'org-1'] });
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: ['nodeApiKeys', 'org-2'] });

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
    await waitFor(() => {
      expect(listSpy).toHaveBeenCalledTimes(1);
    });
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    await u.type(nameInput, 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/boom/)).toBeInTheDocument();
    });
    expect(listSpy).toHaveBeenCalledTimes(1);
  });

  it('every settings.swarm.apiKeys.* key used in NodeApiKeySection.tsx has a matching key in the en locale (TS9)', () => {
    const requiredKeys = [
      'settings.swarm.apiKeys.title',
      'settings.swarm.apiKeys.description',
      'settings.swarm.apiKeys.create',
      'settings.swarm.apiKeys.createTitle',
      'settings.swarm.apiKeys.createDescription',
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
      'settings.swarm.apiKeys.loadError',
      'settings.swarm.apiKeys.secretVisible',
      'settings.swarm.apiKeys.secretHidden',
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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const apiKeysBlock = (enSettings as any).settings?.swarm?.apiKeys ?? {};
    for (const key of requiredKeys) {
      const suffix = key.replace('settings.swarm.apiKeys.', '');
      expect(apiKeysBlock[suffix], `Missing i18n key: ${key}`).toBeDefined();
    }
  });

  it('renders Revoked badge with no action button for revoked keys (TS10)', async () => {
    const keys: NodeApiKey[] = [{
      id: 'k3', organization_id: 'org-1', name: 'Old Key', key_prefix: 'vk_old',
      created_by: null, last_used_at: null, revoked_at: '2026-02-01T00:00:00Z',
      created_at: '2026-01-01T00:00:00Z', node_id: null,
      takeover_count: 0, takeover_window_start: null,
      blocked_at: null, blocked_reason: null,
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    expect(await screen.findByText('Revoked')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Revoke' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Unblock' })).not.toBeInTheDocument();
  });

  it('renders last-used timestamp when present and omits when null (TS11)', async () => {
    const keys: NodeApiKey[] = [
      {
        id: 'k1', organization_id: 'org-1', name: 'Used', key_prefix: 'vk_u',
        created_by: null, last_used_at: '2026-03-15T00:00:00Z', revoked_at: null,
        created_at: '2026-01-01T00:00:00Z', node_id: null,
        takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      {
        id: 'k2', organization_id: 'org-1', name: 'Unused', key_prefix: 'vk_n',
        created_by: null, last_used_at: null, revoked_at: null,
        created_at: '2026-01-01T00:00:00Z', node_id: null,
        takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
    ];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await waitFor(() => {
      expect(screen.getByText(/Last used 2026-03-15/)).toBeInTheDocument();
    });
    expect(screen.queryByText(/Last used 2026-01-01/)).not.toBeInTheDocument();
  });

  it('renders nothing when organizationId is empty string (TS12)', () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    const { container } = renderWith(<NodeApiKeySection organizationId="" />);
    expect(container.innerHTML).toBe('');
  });

  it('unblock mutation error surfaces in the destructive Alert (TS13)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const keys: NodeApiKey[] = [{
      id: 'k2', organization_id: 'org-1', name: 'Blocked', key_prefix: 'vk_b',
      created_by: null, last_used_at: null, revoked_at: null,
      created_at: '2026-01-01T00:00:00Z', node_id: null,
      takeover_count: 0, takeover_window_start: null,
      blocked_at: '2026-01-02T00:00:00Z', blocked_reason: 'Duplicate key use detected',
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    vi.mocked(nodesApi.unblockApiKey).mockRejectedValue(new Error('unblock failed'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(await screen.findByRole('button', { name: 'Unblock' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/unblock failed/)).toBeInTheDocument();
    });
    confirmSpy.mockRestore();
  });

  it('copies secret via navigator.clipboard.writeText when available (TS14)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockResolvedValue({
      api_key: {
        id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      secret: 'vk_SECRET_CLIPBOARD_TEST',
    });
    const writeText = vi.fn().mockResolvedValue(undefined);
    const origClipboard = navigator.clipboard;
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true, writable: true });

    try {
      const { fireEvent } = await import('@testing-library/react');
      renderWith(<NodeApiKeySection organizationId="org-1" />);
      fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
      const nameInput = await screen.findByLabelText('Key Name');
      fireEvent.change(nameInput, { target: { value: 'Test' } });
      fireEvent.click(screen.getByRole('button', { name: 'Create' }));
      await waitFor(() => {
        expect(screen.getByText('••••••••••••••••••••')).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: 'Reveal' }));
      await waitFor(() => {
        expect(screen.getByText('vk_SECRET_CLIPBOARD_TEST')).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
      await waitFor(() => {
        expect(writeText).toHaveBeenCalledWith('vk_SECRET_CLIPBOARD_TEST');
        expect(screen.getByText('Copied!')).toBeInTheDocument();
      });
    } finally {
      Object.defineProperty(navigator, 'clipboard', { value: origClipboard, configurable: true, writable: true });
    }
  });

  it('dialog close via X button clears state when no secret is shown (TS15)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    expect(await screen.findByLabelText('Key Name')).toBeInTheDocument();
    const closeBtn = screen.getByRole('button', { name: 'Close' });
    await u.click(closeBtn);
    await waitFor(() => {
      expect(screen.queryByLabelText('Key Name')).not.toBeInTheDocument();
    });
  });

  it('parseErrorMessage handles string rejection (TS16a)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockRejectedValue('plain failure');
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await u.type(await screen.findByLabelText('Key Name'), 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/plain failure/)).toBeInTheDocument();
    });
  });

  it('parseErrorMessage handles null rejection (TS16b)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockRejectedValue(null);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await u.type(await screen.findByLabelText('Key Name'), 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Failed/)).toBeInTheDocument();
    });
  });

  it('parseErrorMessage handles plain object rejection (TS16c)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockRejectedValue({ code: 'E_DENIED' });
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await u.type(await screen.findByLabelText('Key Name'), 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/E_DENIED/)).toBeInTheDocument();
    });
  });

  it('parseErrorMessage extracts message from JSON error body (TS16d)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockRejectedValue(new Error('{"message":"server denied"}'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await u.type(await screen.findByLabelText('Key Name'), 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/server denied/)).toBeInTheDocument();
    });
  });

  it('parseErrorMessage handles symbol rejection (TS16e)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockRejectedValue(Symbol('boom'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await u.type(await screen.findByLabelText('Key Name'), 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Failed/)).toBeInTheDocument();
    });
  });

  it('parseErrorMessage handles circular reference object (TS16f)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    vi.mocked(nodesApi.createApiKey).mockRejectedValue(circular);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    await u.type(await screen.findByLabelText('Key Name'), 'X');
    await u.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/Failed/)).toBeInTheDocument();
    });
  });

  it('revoke mutation error surfaces in the destructive Alert (TS17)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const keys: NodeApiKey[] = [{
      id: 'k1', organization_id: 'org-1', name: 'Active', key_prefix: 'vk_a',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: 'n1', takeover_count: 0, takeover_window_start: null,
      blocked_at: null, blocked_reason: null,
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    vi.mocked(nodesApi.revokeApiKey).mockRejectedValue(new Error('revoke failed'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    await u.click(await screen.findByRole('button', { name: 'Revoke' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText(/revoke failed/)).toBeInTheDocument();
    });
    confirmSpy.mockRestore();
  });

  it('pressing Enter in name input submits the create form (TS18)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockResolvedValue({
      api_key: {
        id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      secret: 'vk_ENTER_TEST',
    });
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    await u.type(nameInput, 'Enter Key{Enter}');
    await waitFor(() => {
      expect(nodesApi.createApiKey).toHaveBeenCalledWith({ organization_id: 'org-1', name: 'Enter Key' });
    });
  });

  it('renders Blocked badge without tooltip when blocked_reason is null (TS19)', async () => {
    const keys: NodeApiKey[] = [{
      id: 'k4', organization_id: 'org-1', name: 'Blocked No Reason', key_prefix: 'vk_nr',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: 'n1', takeover_count: 0, takeover_window_start: null,
      blocked_at: '2026-01-02T00:00:00Z', blocked_reason: null,
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    expect(await screen.findByText('Blocked')).toBeInTheDocument();
    expect(screen.queryByText('Duplicate key use detected')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unblock' })).toBeInTheDocument();
  });

  it('re-enables the Unblock button after a failed unblock mutation (TS20)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const keys: NodeApiKey[] = [{
      id: 'k2', organization_id: 'org-1', name: 'Blocked', key_prefix: 'vk_b',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: null, takeover_count: 0, takeover_window_start: null,
      blocked_at: '2026-01-02T00:00:00Z', blocked_reason: 'Duplicate',
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    vi.mocked(nodesApi.unblockApiKey).mockRejectedValue(new Error('unblock failed'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    const unblockBtn = await screen.findByRole('button', { name: 'Unblock' });
    await u.click(unblockBtn);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Unblock' })).not.toBeDisabled();
    });
    confirmSpy.mockRestore();
  });

  it('re-enables the Revoke button after a failed revoke mutation (TS21)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const keys: NodeApiKey[] = [{
      id: 'k1', organization_id: 'org-1', name: 'Active', key_prefix: 'vk_a',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: 'n1', takeover_count: 0, takeover_window_start: null,
      blocked_at: null, blocked_reason: null,
    }];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    vi.mocked(nodesApi.revokeApiKey).mockRejectedValue(new Error('revoke failed'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    const revokeBtn = await screen.findByRole('button', { name: 'Revoke' });
    await u.click(revokeBtn);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Revoke' })).not.toBeDisabled();
    });
    confirmSpy.mockRestore();
  });

  it('renders load-error Alert when listApiKeys rejects (TS22)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockRejectedValue(new Error('network down'));
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await waitFor(() => {
      expect(screen.getByText('Failed to load API keys.')).toBeInTheDocument();
    });
  });

  it('hides the Close button when the secret is shown (uncloseable) (TS23)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockResolvedValue({
      api_key: {
        id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      secret: 'vk_UNCLOSEABLE_TEST',
    });
    const { fireEvent } = await import('@testing-library/react');
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    fireEvent.change(nameInput, { target: { value: 'Test' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByText('••••••••••••••••••••')).toBeInTheDocument();
    });
    expect(screen.queryByRole('button', { name: 'Close' })).not.toBeInTheDocument();
  });

  it('does not fire listApiKeys when organizationId is empty (TS24)', () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    renderWith(<NodeApiKeySection organizationId="" />);
    expect(nodesApi.listApiKeys).not.toHaveBeenCalled();
  });

  it('closes create dialog and resets state when organizationId changes (TS25)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    const { fireEvent } = await import('@testing-library/react');
    const { default: user } = await import('@testing-library/user-event');
    const u = user.setup();
    const result = renderWith(<NodeApiKeySection organizationId="org-1" />);
    await u.click(screen.getByRole('button', { name: 'Generate API Key' }));
    expect(await screen.findByLabelText('Key Name')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Key Name'), { target: { value: 'Test' } });
    result.rerender(
      <QueryClientProvider client={queryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-2" /></TooltipProvider>
      </QueryClientProvider>
    );
    await waitFor(() => {
      expect(screen.queryByLabelText('Key Name')).not.toBeInTheDocument();
    });
  });

  it('does not show Copied! feedback when execCommand returns false (TS26)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockResolvedValue({
      api_key: {
        id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      secret: 'vk_EXEC_FAIL_TEST',
    });
    const execCommand = vi.fn(() => false);
    const origExecCommand = document.execCommand;
    document.execCommand = execCommand;
    const origClipboard = navigator.clipboard;
    // @ts-expect-error — assigning undefined to disable clipboard in test
    navigator.clipboard = undefined;
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    try {
      const { fireEvent } = await import('@testing-library/react');
      renderWith(<NodeApiKeySection organizationId="org-1" />);
      fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
      const nameInput = await screen.findByLabelText('Key Name');
      fireEvent.change(nameInput, { target: { value: 'Test' } });
      fireEvent.click(screen.getByRole('button', { name: 'Create' }));
      await waitFor(() => {
        expect(screen.getByText('••••••••••••••••••••')).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith('Failed to copy to clipboard');
      });
      expect(screen.queryByText('Copied!')).not.toBeInTheDocument();
    } finally {
      Object.defineProperty(navigator, 'clipboard', { value: origClipboard, configurable: true });
      document.execCommand = origExecCommand;
      consoleSpy.mockRestore();
    }
  });

  it('handles clipboard writeText rejection gracefully (TS27)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    vi.mocked(nodesApi.createApiKey).mockResolvedValue({
      api_key: {
        id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      secret: 'vk_CLIPBOARD_REJECT_TEST',
    });
    const writeText = vi.fn().mockRejectedValue(new Error('not allowed'));
    const origClipboard = navigator.clipboard;
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true, writable: true });
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    try {
      const { fireEvent } = await import('@testing-library/react');
      renderWith(<NodeApiKeySection organizationId="org-1" />);
      fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
      const nameInput = await screen.findByLabelText('Key Name');
      fireEvent.change(nameInput, { target: { value: 'Test' } });
      fireEvent.click(screen.getByRole('button', { name: 'Create' }));
      await waitFor(() => {
        expect(screen.getByText('••••••••••••••••••••')).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith('Failed to copy to clipboard');
      });
      expect(screen.queryByText('Copied!')).not.toBeInTheDocument();
    } finally {
      Object.defineProperty(navigator, 'clipboard', { value: origClipboard, configurable: true, writable: true });
      consoleSpy.mockRestore();
    }
  });
});
