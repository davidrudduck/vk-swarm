import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
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
import { configApi } from '@/lib/api';

const ACCESS_JWT =
  'eyJhbGciOiJub25lIn0.eyJ0ZXN0X2xhYmVsIjoiU0VOVElORUwtQUNDRVNTLThmMzFjMGQyIn0.sentinel';
const REFRESH_SENTINEL = 'SENTINEL-REFRESH-4b7ae19f';

function storageText(storage: Storage): string {
  const entries: string[] = [];
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i)!;
    entries.push(key, storage.getItem(key) ?? '');
  }
  return entries.join('\n');
}

function assertClean(haystack: string) {
  expect(haystack).not.toContain(ACCESS_JWT);
  expect(haystack).not.toContain(REFRESH_SENTINEL);
}

function scanBrowserSurfaces() {
  assertClean(document.body.textContent ?? '');
  assertClean(storageText(localStorage));
  assertClean(storageText(sessionStorage));
  assertClean(window.location.href);
}

describe('token disclosure', () => {
  beforeEach(() => {
    browserAuthApi.getState.mockReset();
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('scanner detects deliberate JWT leak in DOM and storage', () => {
    localStorage.setItem('leak', ACCESS_JWT);
    render(
      <div>
        {ACCESS_JWT}
        {REFRESH_SENTINEL}
      </div>
    );
    expect(() => assertClean(document.body.textContent ?? '')).toThrow();
    expect(() => assertClean(storageText(localStorage))).toThrow();
    localStorage.removeItem('leak');
  });

  it('unauthorized auth-state with unexpected sentinel fields does not disclose', async () => {
    browserAuthApi.getState.mockResolvedValue({
      authorized: false,
      oauth_available: true,
      access_token: ACCESS_JWT,
      refresh_token: REFRESH_SENTINEL,
    });
    render(<AuthBoundary>protected</AuthBoundary>);
    await waitFor(() =>
      expect(screen.getByTestId('login-shell')).toBeInTheDocument()
    );
    scanBrowserSurfaces();
  });

  it('authorized bootstrap with unexpected sentinel fields does not disclose', async () => {
    browserAuthApi.getState.mockResolvedValue({
      authorized: true,
      oauth_available: true,
      access_token: ACCESS_JWT,
      refresh_token: REFRESH_SENTINEL,
    });
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      const url = String(input);
      if (url.includes('/api/info')) {
        return new Response(
          JSON.stringify({
            success: true,
            data: {
              analytics_user_id: 'probe-user',
              access_token: ACCESS_JWT,
              refresh_token: REFRESH_SENTINEL,
              config: {},
              login_status: { status: 'loggedout' },
              environment: {},
              executors: {},
              capabilities: {},
            },
            error_data: null,
            message: null,
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        );
      }
      return new Response(null, { status: 404 });
    });
    function Probe() {
      const [id, setId] = React.useState('');
      React.useEffect(() => {
        void configApi
          .getConfig()
          .then((info) => setId(info.analytics_user_id));
      }, []);
      return <div data-testid="authorized-probe">{id}</div>;
    }
    render(
      <AuthBoundary>
        <Probe />
      </AuthBoundary>
    );
    await waitFor(() =>
      expect(screen.getByTestId('authorized-probe')).toHaveTextContent(
        'probe-user'
      )
    );
    scanBrowserSurfaces();
  });
});
