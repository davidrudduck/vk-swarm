import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { organizationsApi } from './organizations';
import { ApiError } from './utils';

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

describe('organizationsApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    g.fetch = vi.fn();
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('GETs /v1/organizations with Bearer token and returns organizations array', async () => {
    localStorage.setItem('access_token', 'test-token-123');

    const mockOrganizations = [
      { id: 'org-1', name: 'Organization 1', slug: 'org-1' },
      { id: 'org-2', name: 'Organization 2', slug: 'org-2' },
    ];

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ organizations: mockOrganizations }),
    } as Response);

    const result = await organizationsApi.list();

    const call = mockFetch.mock.calls[0] as unknown as MockFetchCall;
    expect(call[0]).toContain('/v1/organizations');
    expect(call[1].method).toBe('GET');
    expect((call[1].headers as Headers).get('Authorization')).toBe('Bearer test-token-123');

    expect(result).toEqual(mockOrganizations);
  });

  it('throws Error when no access token is available', async () => {
    await expect(organizationsApi.list()).rejects.toThrow('No access token found');
  });

  it('throws ApiError when response is not ok', async () => {
    localStorage.setItem('access_token', 'test-token-123');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 403,
      json: async () => ({ message: 'Forbidden' }),
    } as Response);

    await expect(organizationsApi.list()).rejects.toThrow(ApiError);
  });

  it('returns empty array when no organizations exist', async () => {
    localStorage.setItem('access_token', 'test-token-123');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ organizations: [] }),
    } as Response);

    const result = await organizationsApi.list();

    expect(result).toEqual([]);
  });
});
