import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { ApiError } from '@/lib/api/utils';
import { useRemoteConnectionStatus } from './useRemoteConnectionStatus';
import type { TaskWithAttemptStatus } from 'shared/types';

vi.mock('@/lib/api', () => ({
  tasksApi: { streamConnectionInfo: vi.fn() },
}));

const remoteTask = {
  id: 'task-1',
  shared_task_id: 'shared-1',
} as unknown as TaskWithAttemptStatus;

describe('useRemoteConnectionStatus 503 discrimination (F-2026-08-01-01)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('treats HiveNotConfigured (503 + discriminator) as a quiet disconnected state: no error', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.streamConnectionInfo).mockRejectedValue(
      new ApiError(
        'HiveNotConfigured: This node is not connected to a hive',
        503
      )
    );

    const { result } = renderHook(() =>
      useRemoteConnectionStatus(remoteTask, { refetchInterval: 0 })
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.status).toBe('disconnected');
    expect(result.current.error).toBeNull();
  });

  it('surfaces a plain 503 (hive outage, no discriminator) as an error — an unconditional 503 guard must fail here', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.streamConnectionInfo).mockRejectedValue(
      new ApiError('Service Unavailable', 503)
    );

    const { result } = renderHook(() =>
      useRemoteConnectionStatus(remoteTask, { refetchInterval: 0 })
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.status).toBe('disconnected');
    expect(result.current.error).toBe('Service Unavailable');
  });

  it('surfaces a non-503 error as an error', async () => {
    const { tasksApi } = await import('@/lib/api');
    vi.mocked(tasksApi.streamConnectionInfo).mockRejectedValue(
      new ApiError('server exploded', 500)
    );

    const { result } = renderHook(() =>
      useRemoteConnectionStatus(remoteTask, { refetchInterval: 0 })
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.error).toBe('server exploded');
  });
});
