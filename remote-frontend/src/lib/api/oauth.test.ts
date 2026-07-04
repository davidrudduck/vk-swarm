import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { oauthApi } from './oauth';
import { profileApi } from './profile';

// Mock localStorage if not available
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

describe('OAuth API', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    g.fetch = vi.fn();
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('oauthApi.init() POSTs to /v1/oauth/web/init and returns HandoffInitResponse', async () => {
    const mockResponse = {
      handoff_id: 'test-handoff-123',
      authorize_url: 'https://auth.example.com/authorize?code=abc',
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        success: true,
        data: mockResponse,
      }),
      url: 'http://localhost/v1/oauth/web/init',
    } as Response);

    const result = await oauthApi.init('github', 'http://localhost/callback', 'challenge-123');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/v1/oauth/web/init'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          provider: 'github',
          return_to: 'http://localhost/callback',
          app_challenge: 'challenge-123',
        }),
      })
    );

    expect(result).toEqual(mockResponse);
  });

  it('oauthApi.redeem() POSTs to /v1/oauth/web/redeem and returns HandoffRedeemResponse', async () => {
    const mockResponse = {
      access_token: 'token-abc123',
      refresh_token: 'refresh-def456',
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        success: true,
        data: mockResponse,
      }),
      url: 'http://localhost/v1/oauth/web/redeem',
    } as Response);

    const result = await oauthApi.redeem('handoff-123', 'auth-code-xyz', 'verifier-abc');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/v1/oauth/web/redeem'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          handoff_id: 'handoff-123',
          app_code: 'auth-code-xyz',
          app_verifier: 'verifier-abc',
        }),
      })
    );

    expect(result).toEqual(mockResponse);
  });

  it('oauthApi.logout() POSTs to /v1/oauth/logout with Bearer token and clears localStorage', async () => {
    localStorage.setItem('access_token', 'test-token-123');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 204,
      url: 'http://localhost/v1/oauth/logout',
    } as Response);

    await oauthApi.logout();

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/oauth/logout');
    expect(call[1].method).toBe('POST');
    expect((call[1].headers as Headers).get('Authorization')).toBe('Bearer test-token-123');

    expect(localStorage.getItem('access_token')).toBeNull();
  });

  it('profileApi.get() GETs /v1/profile with Bearer token and returns ProfileResponse', async () => {
    localStorage.setItem('access_token', 'test-token-abc');

    const mockResponse = {
      user_id: 'user-123',
      username: 'testuser',
      email: 'test@example.com',
      providers: [
        {
          provider: 'github',
          username: 'testuser-github',
          display_name: 'Test User',
          email: 'test@github.com',
          avatar_url: 'https://example.com/avatar.jpg',
        },
      ],
    };

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        success: true,
        data: mockResponse,
      }),
      url: 'http://localhost/v1/profile',
    } as Response);

    const result = await profileApi.get();

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/profile');
    expect(call[1].method).toBe('GET');
    expect((call[1].headers as Headers).get('Authorization')).toBe('Bearer test-token-abc');

    expect(result).toEqual(mockResponse);
  });
});
