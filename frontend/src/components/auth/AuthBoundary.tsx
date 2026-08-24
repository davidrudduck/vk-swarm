import { useEffect, useRef, useState } from 'react';

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
  const mountedRef = useRef(true);

  useEffect(() => {
    const stopPolling = () => {
      if (intervalRef.current !== undefined) {
        window.clearInterval(intervalRef.current);
        intervalRef.current = undefined;
      }
      if (deadlineRef.current !== undefined) {
        window.clearTimeout(deadlineRef.current);
        deadlineRef.current = undefined;
      }
    };

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
      .catch(() => {
        if (mountedRef.current) {
          setAuthState({ authorized: false, oauth_available: false });
        }
      });

    return () => {
      mountedRef.current = false;
      stopPolling();
      unsubscribe();
    };
  }, []);

  const startLogin = async () => {
    const returnTo = `${window.location.origin}/api/auth/handoff/complete`;
    const { authorize_url } = await browserAuthApi.startLogin('github', returnTo);
    const popup = window.open(
      authorize_url,
      'hive-oauth',
      'popup,width=600,height=720'
    );
    popupRef.current = popup;

    const stopPolling = () => {
      if (intervalRef.current !== undefined) {
        window.clearInterval(intervalRef.current);
        intervalRef.current = undefined;
      }
      if (deadlineRef.current !== undefined) {
        window.clearTimeout(deadlineRef.current);
        deadlineRef.current = undefined;
      }
    };

    const poll = async () => {
      if (!mountedRef.current || popupRef.current?.closed) {
        stopPolling();
        return;
      }
      const state = await browserAuthApi.getState();
      if (!mountedRef.current) return;
      if (state.authorized) {
        stopPolling();
        setAuthState(state);
      }
    };

    intervalRef.current = window.setInterval(() => {
      void poll();
    }, POLL_INTERVAL_MS);
    deadlineRef.current = window.setTimeout(stopPolling, LOGIN_DEADLINE_MS);
  };

  if (authState?.authorized) return <>{children}</>;

  return (
    <div data-testid="login-shell">
      {authState?.oauth_available !== false && (
        <button type="button" data-testid="login-start" onClick={() => void startLogin()}>
          Log in
        </button>
      )}
    </div>
  );
}
