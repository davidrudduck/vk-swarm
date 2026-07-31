import { afterEach, describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import { ApiError } from '@/lib/api/utils';
import { useAvailableNodes } from './useAvailableNodes';
import { useNodeLogStream } from './useNodeLogStream';

vi.mock('@/lib/api', () => ({
  tasksApi: { availableNodes: vi.fn() },
}));

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return React.createElement(QueryClientProvider, { client }, children);
}

describe('useAvailableNodes with no hive', () => {
  it('does not throw and reports no nodes when the server says HiveNotConfigured', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.availableNodes).mockRejectedValue(
      new ApiError('no hive', 503)
    );

    const { result } = renderHook(() => useAvailableNodes('task-1'), {
      wrapper,
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    // The consumer (CreateAttemptDialog) must be able to render: no throw,
    // and an empty node list rather than undefined-dereference.
    expect(result.current.isError).toBe(false);
    expect(result.current.data?.nodes ?? []).toEqual([]);
  });

  it('still surfaces a real error (non-503) as an error state', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.availableNodes).mockRejectedValue(
      new ApiError('server exploded', 500)
    );

    const { result } = renderHook(() => useAvailableNodes('task-1'), {
      wrapper,
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.isError).toBe(true);
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

    const { result } = renderHook(() => useNodeLogStream('assignment-1'));

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

    const { result } = renderHook(() => useNodeLogStream('assignment-1'));

    await waitFor(() =>
      expect(result.current.connectionType).toBe('disconnected')
    );
    expect(result.current.error).not.toBeNull();
  });
});
