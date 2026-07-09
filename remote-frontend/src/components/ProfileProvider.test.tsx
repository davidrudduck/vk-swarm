import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { ReactNode } from 'react';
import { ProfileProvider, useProfile } from './ProfileProvider';

describe('ProfileProvider', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
    const localStorageMock = {
      getItem: vi.fn(),
      removeItem: vi.fn(),
      setItem: vi.fn(),
      clear: vi.fn(),
      length: 0,
      key: vi.fn(),
    };
    Object.defineProperty(window, 'localStorage', {
      value: localStorageMock,
      writable: true,
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it('should fetch profile with Bearer token from localStorage on mount (signed-in path)', async () => {
    const mockToken = 'test-access-token-xyz';
    const mockProfile = {
      user_id: '123e4567-e89b-12d3-a456-426614174000',
      username: 'testuser',
      email: 'test@example.com',
      providers: [
        {
          provider: 'github',
          username: 'testuser',
          display_name: 'Test User',
          email: 'test@github.com',
          avatar_url: 'https://example.com/avatar.jpg',
        },
      ],
    };

    vi.mocked(localStorage.getItem).mockReturnValueOnce(mockToken);
    vi.mocked(globalThis.fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockProfile,
    } as Response);

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useProfile(), { wrapper });

    expect(result.current.isLoaded).toBe(false);

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      profile: mockProfile,
      isSignedIn: true,
      isLoaded: true,
    });

    // Verify fetch was called with Authorization header
    const fetchCall = vi.mocked(globalThis.fetch).mock.calls[0];
    expect(fetchCall[0]).toContain('/v1/profile');
    expect((fetchCall[1]?.headers as Headers).get('Authorization')).toBe(`Bearer ${mockToken}`);
  });

  it('should not fetch when no token in localStorage (signed-out, no fetch)', async () => {
    vi.mocked(localStorage.getItem).mockReturnValueOnce(null);

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useProfile(), { wrapper });

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      profile: null,
      isSignedIn: false,
      isLoaded: true,
    });

    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('should clear token from localStorage on 401 response', async () => {
    const mockToken = 'expired-token';

    vi.mocked(localStorage.getItem).mockReturnValueOnce(mockToken);
    vi.mocked(globalThis.fetch).mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({}),
    } as Response);

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useProfile(), { wrapper });

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      profile: null,
      isSignedIn: false,
      isLoaded: true,
    });

    expect(vi.mocked(localStorage.removeItem)).toHaveBeenCalledWith('access_token');
  });

  it('should not clear token on network error (transient failure)', async () => {
    const mockToken = 'test-token';

    vi.mocked(localStorage.getItem).mockReturnValueOnce(mockToken);
    vi.mocked(globalThis.fetch).mockRejectedValueOnce(new Error('Network error'));

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useProfile(), { wrapper });

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      profile: null,
      isSignedIn: false,
      isLoaded: true,
    });

    expect(vi.mocked(localStorage.removeItem)).not.toHaveBeenCalled();
  });

  it('should not clear token on non-401 server error (500)', async () => {
    const mockToken = 'test-token';

    vi.mocked(localStorage.getItem).mockReturnValueOnce(mockToken);
    vi.mocked(globalThis.fetch).mockResolvedValueOnce({
      ok: false,
      status: 500,
      json: async () => ({ message: 'Internal server error' }),
    } as Response);

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useProfile(), { wrapper });

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      profile: null,
      isSignedIn: false,
      isLoaded: true,
    });

    expect(vi.mocked(localStorage.removeItem)).not.toHaveBeenCalled();
  });

  it('should have isLoaded false before fetch resolves (loading state)', () => {
    const mockToken = 'test-token';

    vi.mocked(localStorage.getItem).mockReturnValueOnce(mockToken);
    vi.mocked(globalThis.fetch).mockImplementationOnce(
      () => new Promise(() => {}) // Never resolves
    );

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useProfile(), { wrapper });

    expect(result.current).toEqual({
      profile: null,
      isSignedIn: false,
      isLoaded: false,
    });
  });
});
