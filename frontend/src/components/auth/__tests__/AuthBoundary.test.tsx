import React from 'react';
import { fireEvent, render, screen, waitFor, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { browserAuthApi } = vi.hoisted(() => ({
  browserAuthApi: {
    getState: vi.fn(),
    startLogin: vi.fn(),
    logout: vi.fn(),
    disconnectHive: vi.fn(),
  },
}));

vi.mock('@/lib/api/browserAuth', () => ({ browserAuthApi }));

import { AuthBoundary } from '../AuthBoundary';
import { makeRequest, onUnauthorized, notifyUnauthorized } from '@/lib/api/utils';

const authorized = { authorized: true, oauth_available: true };
const unauthorized = { authorized: false, oauth_available: true };

function popupStub() {
  return { closed: false };
}

async function flushPromises() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe('AuthBoundary', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    browserAuthApi.getState.mockReset();
    browserAuthApi.startLogin.mockReset();
    browserAuthApi.getState.mockResolvedValue(unauthorized);
    browserAuthApi.startLogin.mockResolvedValue({
      handoff_id: 'handoff',
      authorize_url: 'https://hive.test/login',
    });
    window.open = vi.fn(() => popupStub()) as never;
    window.close = vi.fn();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('only checks public auth state on unauthorized mount', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    render(<AuthBoundary>protected</AuthBoundary>);

    await flushPromises();
    vi.useRealTimers();
    await waitFor(() => expect(screen.getByTestId('login-shell')).toBeInTheDocument());
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(1);
    expect(fetchSpy).not.toHaveBeenCalledWith(expect.stringContaining('/api/info'));
    expect(fetchSpy).not.toHaveBeenCalledWith(expect.stringContaining('/api/auth/status'));
    expect(fetchSpy).not.toHaveBeenCalledWith(expect.stringContaining('/api/projects'));
    expect(fetchSpy).not.toHaveBeenCalledWith(expect.stringContaining('/api/events'));
    expect(fetchSpy).not.toHaveBeenCalledWith(expect.stringContaining('ws'));
  });

  it('starts GitHub login and polls only getState', async () => {
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByTestId('login-start')).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('login-start'));
    await flushPromises();
    expect(browserAuthApi.startLogin).toHaveBeenCalledTimes(1);
    expect(browserAuthApi.startLogin).toHaveBeenCalledWith(
      'github',
      `${window.location.origin}/api/auth/handoff/complete`
    );
    expect(window.open).toHaveBeenCalledWith(
      'https://hive.test/login',
      'hive-oauth',
      'popup,width=600,height=720'
    );
    await act(async () => {
      vi.advanceTimersByTime(1000);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(2);
  });

  it('keeps exactly one poll interval after a second login click', async () => {
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    const loginStart = screen.getByTestId('login-start');

    fireEvent.click(loginStart);
    await flushPromises();
    fireEvent.click(loginStart);
    await flushPromises();

    const callsBeforePoll = browserAuthApi.getState.mock.calls.length;
    await act(async () => {
      vi.advanceTimersByTime(1000);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(browserAuthApi.getState).toHaveBeenCalledTimes(callsBeforePoll + 1);
  });

  it('mounts children only after a successful poll', async () => {
    browserAuthApi.getState.mockImplementation(async () =>
      browserAuthApi.getState.mock.calls.length < 3 ? unauthorized : authorized
    );
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByTestId('login-start')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('login-start'));
    await flushPromises();

    await act(async () => {
      vi.advanceTimersByTime(1000);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByTestId('login-shell')).toBeInTheDocument();
    await act(async () => {
      vi.advanceTimersByTime(1000);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(3);
    expect(screen.getByText('protected')).toBeInTheDocument();
    const callsAfterAuthorization = browserAuthApi.getState.mock.calls.length;
    await act(async () => vi.advanceTimersByTime(5000));
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(callsAfterAuthorization);
  });

  it('stops polling when the popup closes', async () => {
    const popup = popupStub();
    window.open = vi.fn(() => popup) as never;
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByTestId('login-start')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('login-start'));
    await flushPromises();
    expect(window.open).toHaveBeenCalled();
    popup.closed = true;
    await act(async () => {
      vi.advanceTimersByTime(1000);
      await Promise.resolve();
    });
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId('login-shell')).toBeInTheDocument();
  });

  it('stops polling at the login deadline', async () => {
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByTestId('login-start')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('login-start'));
    await flushPromises();
    expect(window.open).toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(10 * 60 * 1000));
    const callsAtDeadline = browserAuthApi.getState.mock.calls.length;
    await act(async () => vi.advanceTimersByTime(1000));
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(callsAtDeadline);
    expect(screen.getByTestId('login-shell')).toBeInTheDocument();
  });

  it('clears polling on unmount without closing the popup', async () => {
    const close = vi.fn();
    const popup = { closed: false, close };
    window.open = vi.fn(() => popup) as never;
    const { unmount } = render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByTestId('login-start')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('login-start'));
    await flushPromises();
    expect(window.open).toHaveBeenCalled();
    unmount();
    const callsAfterUnmount = browserAuthApi.getState.mock.calls.length;
    await act(async () => vi.advanceTimersByTime(10 * 60 * 1000));
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(callsAfterUnmount);
    expect(close).not.toHaveBeenCalled();
    expect(window.close).not.toHaveBeenCalled();
  });

  it('keeps the live auth effect under StrictMode', async () => {
    render(
      <React.StrictMode>
        <AuthBoundary>protected</AuthBoundary>
      </React.StrictMode>
    );
    await flushPromises();
    expect(screen.getByTestId('login-shell')).toBeInTheDocument();
  });

  it('does not open a popup or restart polling after unmount during login', async () => {
    let resolveStartLogin: (value: { handoff_id: string; authorize_url: string }) => void;
    browserAuthApi.startLogin.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveStartLogin = resolve;
        })
    );
    const { unmount } = render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    fireEvent.click(screen.getByTestId('login-start'));
    unmount();

    resolveStartLogin!({ handoff_id: 'handoff', authorize_url: 'https://hive.test/login' });
    await flushPromises();
    expect(window.open).not.toHaveBeenCalled();
    const callsAfterUnmount = browserAuthApi.getState.mock.calls.length;
    await act(async () => vi.advanceTimersByTime(10 * 60 * 1000));
    expect(browserAuthApi.getState).toHaveBeenCalledTimes(callsAfterUnmount);
  });

  it('hides login-start when OAuth is unavailable', async () => {
    browserAuthApi.getState.mockResolvedValue({ authorized: false, oauth_available: false });
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByTestId('login-shell')).toBeInTheDocument();
    expect(screen.queryByTestId('login-start')).not.toBeInTheDocument();
  });

  it('tears down children when unauthorized is notified', async () => {
    browserAuthApi.getState.mockResolvedValue(authorized);
    render(<AuthBoundary>protected</AuthBoundary>);
    await flushPromises();
    expect(screen.getByText('protected')).toBeInTheDocument();
    await act(async () => {
      notifyUnauthorized();
    });
    await flushPromises();
    expect(screen.getByTestId('login-shell')).toBeInTheDocument();
    expect(screen.queryByText('protected')).not.toBeInTheDocument();
  });

  it('notifies once and returns a 401 response unchanged', async () => {
    const response = { status: 401, ok: false } as Response;
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(response);
    const handler = vi.fn();
    const unsubscribe = onUnauthorized(handler);

    await expect(makeRequest('/any')).resolves.toBe(response);
    expect(handler).toHaveBeenCalledTimes(1);
    unsubscribe();
  });
});
