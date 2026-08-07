import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { ApiError } from '@/lib/api/utils';
import { useDiffStream } from './useDiffStream';

vi.mock('@/lib/api', () => ({
  tasksApi: { availableNodes: vi.fn(), streamConnectionInfo: vi.fn() },
}));

// The WS stream itself is out of scope here; we only pin the connection-info
// fetch error handling.
vi.mock('./useJsonPatchWsStream', () => ({
  useJsonPatchWsStream: () => ({ data: undefined, error: null }),
}));

describe('useDiffStream 503 discrimination (F-2026-08-01-01)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('treats HiveNotConfigured (503 + discriminator) as quiet: no error surfaced', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.streamConnectionInfo).mockRejectedValue(
      new ApiError(
        'HiveNotConfigured: This node is not connected to a hive',
        503
      )
    );

    const { result } = renderHook(() =>
      useDiffStream(null, true, undefined, { taskId: 'task-1' })
    );

    await waitFor(() =>
      expect(vi.mocked(tasksApi.streamConnectionInfo)).toHaveBeenCalledTimes(1)
    );
    await waitFor(() => expect(result.current.connectionType).toBeNull());
    expect(result.current.error).toBeNull();
    expect(result.current.diffs).toEqual([]);
  });

  it('surfaces a plain 503 (hive outage, no discriminator) as an error — an unconditional 503 guard must fail here', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.streamConnectionInfo).mockRejectedValue(
      // Same status, but no HiveNotConfigured prefix: this is a forwarded
      // upstream outage and MUST NOT be swallowed.
      new ApiError('Service Unavailable', 503)
    );

    const { result } = renderHook(() =>
      useDiffStream(null, true, undefined, { taskId: 'task-1' })
    );

    await waitFor(() =>
      expect(result.current.error).toBe('Service Unavailable')
    );
  });

  it('surfaces a non-503 error as an error', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.streamConnectionInfo).mockRejectedValue(
      new ApiError('server exploded', 500)
    );

    const { result } = renderHook(() =>
      useDiffStream(null, true, undefined, { taskId: 'task-1' })
    );

    await waitFor(() => expect(result.current.error).toBe('server exploded'));
  });
});
