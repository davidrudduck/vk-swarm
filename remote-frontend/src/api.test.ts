import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { getInvitation, acceptInvitation, initOAuth, redeemOAuth } from './api';
import { ApiError } from './lib/api/utils';

const g = globalThis as Record<string, unknown>;
if (!g.localStorage) {
  const store: Record<string, string> = {};
  g.localStorage = {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      Object.keys(store).forEach((key) => delete store[key]);
    },
    key: (index: number) => Object.keys(store)[index] || null,
    length: Object.keys(store).length,
  };
}

interface MockFetchCall {
  [0]: string;
  [1]: RequestInit;
}

describe('api.ts - getInvitation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    g.fetch = vi.fn();
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('GETs /v1/invitations/:token and returns Invitation', async () => {
    const mockInvitation = {
      id: 'inv-123',
      organization_slug: 'test-org',
      organization_name: 'Test Org',
      role: 'admin',
      expires_at: '2026-08-01T00:00:00Z',
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockInvitation,
    } as Response);

    const result = await getInvitation('test-token-123');

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/invitations/test-token-123');
    expect(result).toEqual(mockInvitation);
  });

  it('encodes token in URL', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: 'inv-123' }),
    } as Response);

    await getInvitation('token/with/slashes');

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/invitations/token%2Fwith%2Fslashes');
  });

  it('throws ApiError when response is not ok', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 404,
      json: async () => ({ message: 'Not found' }),
    } as Response);

    await expect(getInvitation('invalid-token')).rejects.toThrow(ApiError);
  });

  it('passes AbortSignal to makeRequest', async () => {
    const controller = new AbortController();
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: 'inv-123' }),
    } as Response);

    await getInvitation('test-token', controller.signal);

    expect(mockFetch).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        signal: expect.anything(),
      })
    );
  });
});

describe('api.ts - acceptInvitation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    g.fetch = vi.fn();
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('POSTs to /v1/invitations/:token/accept with Bearer token', async () => {
    const mockResponse = {
      organization_id: 'org-123',
      organization_slug: 'test-org',
      role: 'admin',
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockResponse,
    } as Response);

    const result = await acceptInvitation('test-token', 'access-token-123');

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/invitations/test-token/accept');
    expect(call[1].method).toBe('POST');
    expect((call[1].headers as Headers).get('Authorization')).toBe('Bearer access-token-123');

    expect(result).toEqual(mockResponse);
  });

  it('encodes token in URL', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ organization_id: 'org-123' }),
    } as Response);

    await acceptInvitation('token/with/slashes', 'access-token');

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/invitations/token%2Fwith%2Fslashes/accept');
  });

  it('throws ApiError when response is not ok', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 410,
      json: async () => ({ message: 'Invitation expired' }),
    } as Response);

    await expect(acceptInvitation('expired-token', 'access-token')).rejects.toThrow(ApiError);
  });

  it('passes AbortSignal to makeRequest', async () => {
    const controller = new AbortController();
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ organization_id: 'org-123' }),
    } as Response);

    await acceptInvitation('test-token', 'access-token', controller.signal);

    expect(mockFetch).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        signal: expect.anything(),
      })
    );
  });
});

describe('api.ts - re-exports', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    g.fetch = vi.fn();
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('initOAuth is re-exported from oauthApi.init', async () => {
    const mockResponse = {
      handoff_id: 'handoff-123',
      authorize_url: 'https://auth.example.com/authorize',
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockResponse,
    } as Response);

    const result = await initOAuth('github', 'http://localhost/callback', 'challenge-123');

    expect(result).toEqual(mockResponse);
  });

  it('redeemOAuth is re-exported from oauthApi.redeem', async () => {
    const mockResponse = {
      access_token: 'token-123',
      refresh_token: 'refresh-456',
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockResponse,
    } as Response);

    const result = await redeemOAuth('handoff-123', 'app-code', 'verifier');

    expect(result).toEqual(mockResponse);
  });
});
