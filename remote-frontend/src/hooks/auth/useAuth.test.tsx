import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { ReactNode } from 'react';
import { ProfileProvider } from '@/components/ProfileProvider';
import { useAuth } from './useAuth';

describe('useAuth', () => {
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

  it('should return signed-in state with userId when profile fetch succeeds (200)', async () => {
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

    vi.mocked(localStorage.getItem).mockReturnValue(mockToken);
    vi.mocked(globalThis.fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockProfile,
    } as Response);

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useAuth(), { wrapper });

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      isSignedIn: true,
      isLoaded: true,
      userId: '123e4567-e89b-12d3-a456-426614174000',
    });
  });

  it('should return signed-out state with null userId when profile fetch returns 401', async () => {
    const mockToken = 'expired-token';

    vi.mocked(localStorage.getItem).mockReturnValue(mockToken);
    vi.mocked(globalThis.fetch).mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({}),
    } as Response);

    const wrapper = ({ children }: { children: ReactNode }) => (
      <ProfileProvider>{children}</ProfileProvider>
    );

    const { result } = renderHook(() => useAuth(), { wrapper });

    await waitFor(() => {
      expect(result.current.isLoaded).toBe(true);
    });

    expect(result.current).toEqual({
      isSignedIn: false,
      isLoaded: true,
      userId: null,
    });
  });

  it('should throw when called outside ProfileProvider', () => {
    expect(() => {
      renderHook(() => useAuth());
    }).toThrow('useProfile must be used within a ProfileProvider');
  });
});
