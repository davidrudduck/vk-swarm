import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';

// Mock react-i18next — assertions match raw keys
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

// Mock NiceModal (spies via vi.hoisted — never close a factory over a hoisted import)
const { mockResolve, mockHide, mockRemove } = vi.hoisted(() => ({
  mockResolve: vi.fn(),
  mockHide: vi.fn(),
  mockRemove: vi.fn(),
}));
vi.mock('@ebay/nice-modal-react', () => ({
  useModal: () => ({
    visible: true,
    resolve: mockResolve,
    hide: mockHide,
    remove: mockRemove,
  }),
  create: (Component: React.ComponentType) => Component,
  default: {
    create: (Component: React.ComponentType) => Component,
  },
}));

// Mock defineModal (sibling TaskFormSheet precedent)
vi.mock('@/lib/modals', () => ({
  defineModal: (Component: React.ComponentType) => Component,
}));

// Mock useAuthStatus: spy records the `enabled` argument, returns a mutable result
const { authStatusSpy, authStatusResult } = vi.hoisted(() => {
  const authStatusResult: {
    data: { logged_in: boolean; profile?: { username: string } };
    isError: boolean;
  } = { data: { logged_in: false }, isError: false };
  return {
    authStatusResult,
    authStatusSpy: vi.fn(() => authStatusResult),
  };
});
vi.mock('@/hooks/auth/useAuthStatus', () => ({
  useAuthStatus: authStatusSpy,
}));

// Mock useAuthMutations: capture the options object the component passes
vi.mock('@/hooks/auth/useAuthMutations', () => ({
  useAuthMutations: (options: {
    onInitSuccess: (data: { authorize_url: string }) => void;
  }) => ({
    initHandoff: {
      mutate: () =>
        options.onInitSuccess({
          authorize_url: 'http://hive.test/v1/oauth/github/start?handoff_id=x',
        }),
    },
  }),
}));

// Mock ConfigProvider
const { mockReloadSystem } = vi.hoisted(() => ({
  mockReloadSystem: vi.fn(),
}));
vi.mock('@/components/ConfigProvider', () => ({
  useUserSystem: () => ({ reloadSystem: mockReloadSystem }),
}));

import { OAuthDialog, POLL_DEADLINE_MS } from '../OAuthDialog';

function lastAuthStatusEnabled(): boolean {
  const calls = authStatusSpy.mock.calls as unknown as [
    { enabled: boolean },
  ][];
  return calls[calls.length - 1][0].enabled;
}

function enterPolling() {
  const result = render(<OAuthDialog />);
  fireEvent.click(screen.getByText('oauth.continueWithGitHub'));
  return result;
}

describe('OAuthDialog bounded polling deadline', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    authStatusResult.data = { logged_in: false };
    authStatusResult.isError = false;
    window.open = vi.fn(() => ({ closed: false, close: vi.fn() })) as never;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('keeps waiting and polling before the deadline', () => {
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1000));

    expect(screen.getByText('oauth.waitingTitle')).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(true);
  });

  it('renders the localized timeout error and stops polling past the deadline', () => {
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1000));
    act(() => vi.advanceTimersByTime(2000));

    expect(screen.getByText('oauth.timeoutError')).toBeInTheDocument();
    expect(screen.getByText('oauth.tryAgain')).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(false);
  });

  it('returns to provider select when tryAgain is clicked after timeout', () => {
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS + 1000));
    fireEvent.click(screen.getByText('oauth.tryAgain'));

    expect(screen.getByText('oauth.title')).toBeInTheDocument();
    expect(screen.getByText('oauth.continueWithGitHub')).toBeInTheDocument();
    expect(screen.getByText('oauth.continueWithGoogle')).toBeInTheDocument();
  });

  it('resolves success before the deadline without ever showing the timeout error', () => {
    authStatusResult.data = {
      logged_in: true,
      profile: { username: 'u' },
    };

    enterPolling();

    expect(mockReloadSystem).toHaveBeenCalled();
    expect(screen.queryByText('oauth.timeoutError')).not.toBeInTheDocument();

    act(() => vi.advanceTimersByTime(1500));

    expect(mockResolve).toHaveBeenCalled();
    expect(mockHide).toHaveBeenCalled();
    expect(screen.queryByText('oauth.timeoutError')).not.toBeInTheDocument();
  });

  it('clears the deadline timer on unmount', () => {
    const { unmount } = enterPolling();

    expect(vi.getTimerCount()).toBeGreaterThan(0);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
