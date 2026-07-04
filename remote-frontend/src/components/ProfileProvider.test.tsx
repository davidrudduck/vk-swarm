import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { ReactNode } from 'react';
import { ProfileProvider, useProfile } from './ProfileProvider';

describe('ProfileProvider', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('should expose profile context with loaded state after successful fetch', async () => {
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

    expect(globalThis.fetch).toHaveBeenCalledWith('/v1/profile', {
      credentials: 'include',
    });
  });

  it('should expose profile context with unauthenticated state on 401 error', async () => {
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
  });

  it('should have isLoaded false before fetch resolves', () => {
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

  it('should throw error when useProfile is used outside ProfileProvider', () => {
    expect(() => {
      renderHook(() => useProfile());
    }).toThrow('useProfile must be used within a ProfileProvider');
  });
});
