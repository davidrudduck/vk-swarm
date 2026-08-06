import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';

// Mock react-i18next — assertions match raw keys. The translator is swappable
// per test (with a fresh identity) so the tRef deadline behavior is testable.
const { translatorRef } = vi.hoisted(() => ({
  translatorRef: { current: (key: string) => key },
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: translatorRef.current,
  }),
}));

// Mock NiceModal (spies via vi.hoisted — never close a factory over a hoisted import)
const { mockResolve, mockHide, mockRemove, modalState } = vi.hoisted(() => ({
  mockResolve: vi.fn(),
  mockHide: vi.fn(),
  mockRemove: vi.fn(),
  modalState: { visible: true },
}));
vi.mock('@ebay/nice-modal-react', () => ({
  useModal: () => ({
    visible: modalState.visible,
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
    data: { logged_in: boolean; profile?: { username: string } } | undefined;
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

// Mock useAuthMutations: capture the options object the component passes and
// let each test choose whether init succeeds or fails.
const { initBehavior } = vi.hoisted(() => ({
  initBehavior: { mode: 'success' as 'success' | 'error' },
}));
vi.mock('@/hooks/auth/useAuthMutations', () => ({
  useAuthMutations: (options: {
    onInitSuccess: (data: { authorize_url: string }) => void;
    onInitError: (error: unknown) => void;
  }) => ({
    initHandoff: {
      mutate: () => {
        if (initBehavior.mode === 'error') {
          options.onInitError(new Error('handoff init exploded'));
        } else {
          options.onInitSuccess({
            authorize_url:
              'http://hive.test/v1/oauth/github/start?handoff_id=x',
          });
        }
      },
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

type PopupStub = { closed: boolean; close: ReturnType<typeof vi.fn> };

function stubPopup(): PopupStub {
  const popup: PopupStub = {
    closed: false,
    close: vi.fn(() => {
      popup.closed = true;
    }),
  };
  window.open = vi.fn(() => popup) as never;
  return popup;
}

function enterPolling(provider: 'GitHub' | 'Google' = 'GitHub') {
  const result = render(<OAuthDialog />);
  fireEvent.click(screen.getByText(`oauth.continueWith${provider}`));
  return result;
}

function resetTestState() {
  vi.useFakeTimers();
  vi.clearAllMocks();
  translatorRef.current = (key: string) => key;
  modalState.visible = true;
  initBehavior.mode = 'success';
  authStatusResult.data = { logged_in: false };
  authStatusResult.isError = false;
  window.open = vi.fn(() => ({ closed: false, close: vi.fn() })) as never;
}

describe('OAuthDialog bounded polling deadline', () => {
  beforeEach(resetTestState);

  afterEach(() => {
    vi.useRealTimers();
  });

  it('keeps waiting and polling before the deadline', () => {
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1000));

    expect(screen.getByText('oauth.waitingTitle')).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(true);
  });

  it('fires exactly at the deadline, not one millisecond before', () => {
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1));
    expect(screen.getByText('oauth.waitingTitle')).toBeInTheDocument();

    act(() => vi.advanceTimersByTime(1));
    expect(screen.getByText('oauth.timeoutError')).toBeInTheDocument();
  });

  it('renders the localized timeout error and stops polling past the deadline', () => {
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1000));
    act(() => vi.advanceTimersByTime(2000));

    expect(screen.getByText('oauth.timeoutError')).toBeInTheDocument();
    expect(screen.getByText('oauth.tryAgain')).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(false);
  });

  it('closes the popup window when the deadline fires', () => {
    const popup = stubPopup();
    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS));

    expect(popup.close).toHaveBeenCalledTimes(1);
  });

  it('does not reset the deadline when the translator changes mid-wait', () => {
    const popup = stubPopup();
    const { rerender } = enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1000));

    // Language switch: new t identity flows in via re-render. The deadline
    // effect depends only on isPolling, so the timer must NOT restart.
    translatorRef.current = (key: string) => `es:${key}`;
    rerender(<OAuthDialog />);

    act(() => vi.advanceTimersByTime(1000));

    // Fires at the ORIGINAL deadline (1s after the switch, not 120s after)
    // and resolves the message with the CURRENT translator (tRef behavior).
    expect(screen.getByText('es:oauth.timeoutError')).toBeInTheDocument();
    expect(popup.close).toHaveBeenCalled();
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

  it('success cancels the deadline: no timeout error even past the deadline', () => {
    const popup = stubPopup();
    authStatusResult.data = {
      logged_in: true,
      profile: { username: 'u' },
    };

    enterPolling();

    act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS + 1000));

    expect(screen.queryByText('oauth.timeoutError')).not.toBeInTheDocument();
    expect(popup.close).toHaveBeenCalled();
  });

  it('clears the deadline timer on unmount', () => {
    const { unmount } = enterPolling();

    expect(vi.getTimerCount()).toBeGreaterThan(0);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe('OAuthDialog error and lifecycle paths', () => {
  beforeEach(resetTestState);

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows the init error message when the handoff init fails', () => {
    initBehavior.mode = 'error';
    enterPolling();

    expect(screen.getByText('handoff init exploded')).toBeInTheDocument();
    expect(screen.getByText('oauth.errorTitle')).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(false);
  });

  it('shows a status-check error and stops polling when the status query errors', () => {
    const { rerender } = enterPolling();

    authStatusResult.isError = true;
    rerender(<OAuthDialog />);

    expect(
      screen.getByText('Failed to check OAuth status')
    ).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(false);
  });

  it('errors when the popup is closed before authentication completes', () => {
    const popup = stubPopup();
    const { rerender } = enterPolling();

    popup.closed = true;
    // Fresh statusData identity so the status-monitor effect re-runs.
    authStatusResult.data = { logged_in: false };
    rerender(<OAuthDialog />);

    expect(
      screen.getByText(
        'OAuth window was closed before completing authentication'
      )
    ).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(false);
  });

  it('cancel during waiting closes the popup and resolves null', () => {
    const popup = stubPopup();
    enterPolling();

    fireEvent.click(screen.getByText('buttons.cancel'));

    expect(popup.close).toHaveBeenCalled();
    expect(mockResolve).toHaveBeenCalledWith(null);
    expect(mockHide).toHaveBeenCalled();
    expect(lastAuthStatusEnabled()).toBe(false);
  });

  it('back during waiting returns to select and closes the popup without resolving', () => {
    const popup = stubPopup();
    enterPolling();

    fireEvent.click(screen.getByText('oauth.back'));

    expect(popup.close).toHaveBeenCalled();
    expect(mockResolve).not.toHaveBeenCalled();
    expect(screen.getByText('oauth.title')).toBeInTheDocument();
  });

  it('enters waiting via the Google provider button too', () => {
    enterPolling('Google');

    expect(screen.getByText('oauth.waitingTitle')).toBeInTheDocument();
    expect(lastAuthStatusEnabled()).toBe(true);
  });

  it('stops polling and closes the popup when the modal becomes invisible', () => {
    const popup = stubPopup();
    const { rerender } = enterPolling();

    modalState.visible = false;
    rerender(<OAuthDialog />);

    expect(popup.close).toHaveBeenCalled();
    expect(lastAuthStatusEnabled()).toBe(false);
  });
});
