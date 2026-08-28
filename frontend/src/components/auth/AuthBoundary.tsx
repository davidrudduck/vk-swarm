import { useCallback, useEffect, useRef, useState } from 'react';

import type { BrowserAuthState } from 'shared/types';

import { browserAuthApi } from '@/lib/api/browserAuth';
import { onUnauthorized } from '@/lib/api/utils';

const POLL_INTERVAL_MS = 1000;
const LOGIN_DEADLINE_MS = 10 * 60 * 1000;

type Props = { children: React.ReactNode };

export function AuthBoundary({ children }: Props) {
  const [authState, setAuthState] = useState<BrowserAuthState | null>(null);
  const intervalRef = useRef<number | undefined>(undefined);
  const deadlineRef = useRef<number | undefined>(undefined);
  const popupRef = useRef<Window | null>(null);
  const mountedRef = useRef(false);

  const stopPolling = useCallback(() => {
    if (intervalRef.current !== undefined) {
      window.clearInterval(intervalRef.current);
      intervalRef.current = undefined;
    }
    if (deadlineRef.current !== undefined) {
      window.clearTimeout(deadlineRef.current);
      deadlineRef.current = undefined;
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;

    const unsubscribe = onUnauthorized(() => {
      stopPolling();
      if (mountedRef.current) {
        setAuthState((current) => ({
          authorized: false,
          oauth_available: current?.oauth_available ?? false,
        }));
      }
    });

    browserAuthApi
      .getState()
      .then((state) => {
        if (mountedRef.current) setAuthState(state);
      })
      .catch((err: unknown) => {
        // Offline / node-down: fall back to the fail-closed state, but leave a
        // trace so a blank shell is diagnosable from the console.
        console.warn('[AuthBoundary] failed to load auth state:', err);
        if (mountedRef.current) {
          setAuthState({ authorized: false, oauth_available: false });
        }
      });

    return () => {
      mountedRef.current = false;
      stopPolling();
      unsubscribe();
    };
  }, [stopPolling]);

  const startLogin = async () => {
    try {
      const returnTo = `${window.location.origin}/api/auth/handoff/complete`;
      const { authorize_url } = await browserAuthApi.startLogin(
        'github',
        returnTo
      );
      if (!mountedRef.current) return;

      const popup = window.open(
        authorize_url,
        'hive-oauth',
        'popup,width=600,height=720'
      );
      if (!popup) {
        // Popup blocked (disabled popups, gesture lost, ...). Warn — the shell
        // below offers the button again, so the user can retry deliberately.
        // Do not clobber an already-open popup's ref: a blocked re-click
        // would otherwise make `closed` permanently false.
        console.warn('[AuthBoundary] login popup was blocked by the browser');
        return;
      }
      popupRef.current = popup;
      stopPolling();

      const poll = async () => {
        if (!mountedRef.current) {
          stopPolling();
          return;
        }
        const closed = Boolean(popupRef.current?.closed);
        try {
          const state = await browserAuthApi.getState();
          if (!mountedRef.current) return;
          if (state.authorized) {
            stopPolling();
            setAuthState(state);
            return;
          }
          if (closed) {
            stopPolling();
          }
        } catch (err: unknown) {
          // Transient poll failure (network flap, node restart). Keep polling
          // while the popup is open — the next tick usually recovers.
          console.warn('[AuthBoundary] auth state poll failed:', err);
          if (!mountedRef.current || closed) {
            stopPolling();
          }
        }
      };

      intervalRef.current = window.setInterval(() => {
        void poll();
      }, POLL_INTERVAL_MS);
      deadlineRef.current = window.setTimeout(stopPolling, LOGIN_DEADLINE_MS);
    } catch (err: unknown) {
      // Hive unreachable or startLogin failed: stay on the login shell, no poll.
      console.error('[AuthBoundary] login could not start:', err);
    }
  };

  if (authState?.authorized) return <>{children}</>;

  return (
    <div data-testid="login-shell">
      {authState?.oauth_available !== false && (
        <button
          type="button"
          data-testid="login-start"
          onClick={() => void startLogin()}
        >
          Log in
        </button>
      )}
    </div>
  );
}
