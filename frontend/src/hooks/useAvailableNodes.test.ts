import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import { ApiError } from '@/lib/api/utils';
import { useAvailableNodes } from './useAvailableNodes';
import { useNodeLogStream } from './useNodeLogStream';

vi.mock('@/lib/api', () => ({
  tasksApi: { availableNodes: vi.fn() },
}));

// NOTE (F-2026-08-01-02): retry is deliberately NOT disabled here. The hook's
// contract is that HiveNotConfigured RESOLVES (empty node list) rather than
// throwing, which is what suppresses TanStack Query's retry loop. A wrapper
// with `retry: false` would make that suppression unobservable — the retry
// assertions below only mean something because retries are enabled.
function makeWrapper() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: 2, retryDelay: 0 } },
  });
  return function wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client }, children);
  };
}

describe('useAvailableNodes with no hive', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('resolves quietly with no nodes and WITHOUT retrying when the server says HiveNotConfigured', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.availableNodes).mockRejectedValue(
      // The REAL server message — status alone is not sufficient, because an
      // upstream 503 from a hive OUTAGE is forwarded verbatim.
      new ApiError(
        'HiveNotConfigured: This node is not connected to a hive',
        503
      )
    );

    const { result } = renderHook(() => useAvailableNodes('task-1'), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    // The consumer (CreateAttemptDialog) must be able to render: no throw,
    // and an empty node list rather than undefined-dereference.
    expect(result.current.isError).toBe(false);
    expect(result.current.data?.nodes ?? []).toEqual([]);
    // The queryFn resolved on the first call, so the (enabled) retry loop
    // never fired. If the hook re-threw instead of resolving, this would be 3.
    expect(vi.mocked(tasksApi.availableNodes)).toHaveBeenCalledTimes(1);
  });

  it('still surfaces a real error (non-503) as an error state, WITH retries', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.availableNodes).mockRejectedValue(
      new ApiError('server exploded', 500)
    );

    const { result } = renderHook(() => useAvailableNodes('task-1'), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    // Control: proves retries are actually enabled in this wrapper (initial
    // attempt + 2 retries), so the 1-call assertion above is meaningful.
    expect(vi.mocked(tasksApi.availableNodes)).toHaveBeenCalledTimes(3);
  });
});

describe('useNodeLogStream on a node with no hive', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('treats the SPA fallback (200 text/html) for /v1/* as "no stream", not an error', async () => {
    // /v1/* is the hive's namespace and is unregistered on a node with no
    // hive: the request falls through to the SPA catch-all, returning
    // `200 text/html` (index.html) rather than JSON. The hook must not try
    // to JSON-parse that and must not surface it as an error.
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('<!doctype html><html></html>', {
        status: 200,
        headers: { 'content-type': 'text/html' },
      })
    );

    const { result } = renderHook(() =>
      useNodeLogStream('assignment-1', 'process-1')
    );

    await waitFor(() =>
      expect(result.current.connectionType).toBe('disconnected')
    );
    expect(result.current.error).toBeNull();
    expect(result.current.logs).toEqual([]);
  });

  it('still surfaces a real failure (500 text/plain) as an error, not a swallowed "no stream"', async () => {
    // A genuine failure (e.g. a misconfigured proxy returning 500 text/plain)
    // must NOT be treated as the hive-absent case: !response.ok is checked
    // before the content-type guard, so this must still throw and surface.
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('internal error', {
        status: 500,
        headers: { 'content-type': 'text/plain' },
      })
    );

    const { result } = renderHook(() =>
      useNodeLogStream('assignment-1', 'process-1')
    );

    await waitFor(() =>
      expect(result.current.connectionType).toBe('disconnected')
    );
    expect(result.current.error).not.toBeNull();
  });
});
