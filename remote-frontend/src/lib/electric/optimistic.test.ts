import { describe, it, expect, vi, beforeEach } from 'vitest';
import { optimisticDelete, optimisticUpdate } from './optimistic';

describe('optimistic mutations (SC8)', () => {
  let queryClient: { setQueryData: ReturnType<typeof vi.fn>; getQueryData: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    queryClient = {
      setQueryData: vi.fn(),
      getQueryData: vi.fn(() => undefined),
    };
  });

  it('optimisticDelete calls setQueryData to remove item', async () => {
    await optimisticDelete(
      queryClient as unknown as Parameters<typeof optimisticDelete>[0],
      ['task-assignments'],
      'a1',
      async () => {},
    );
    expect(queryClient.setQueryData).toHaveBeenCalledTimes(1);
  });

  it('optimisticUpdate calls setQueryData to patch item', async () => {
    await optimisticUpdate(
      queryClient as unknown as Parameters<typeof optimisticUpdate>[0],
      ['task-assignments'],
      'a1',
      { execution_status: 'completed' },
      async () => {},
    );
    expect(queryClient.setQueryData).toHaveBeenCalledTimes(1);
  });

  it('optimisticDelete rolls back on error', async () => {
    queryClient.getQueryData = vi.fn(() => [{ id: 'a1' }, { id: 'a2' }]);

    await expect(
      optimisticDelete(
        queryClient as unknown as Parameters<typeof optimisticDelete>[0],
        ['task-assignments'],
        'a1',
        async () => {
          throw new Error('network error');
        },
      ),
    ).rejects.toThrow('network error');
    expect(queryClient.setQueryData).toHaveBeenCalledTimes(2);
  });

  it('optimisticUpdate rolls back on error', async () => {
    queryClient.getQueryData = vi.fn(() => [{ id: 'a1' }, { id: 'a2' }]);

    await expect(
      optimisticUpdate(
        queryClient as unknown as Parameters<typeof optimisticUpdate>[0],
        ['task-assignments'],
        'a1',
        { execution_status: 'completed' },
        async () => {
          throw new Error('network error');
        },
      ),
    ).rejects.toThrow('network error');
    expect(queryClient.setQueryData).toHaveBeenCalledTimes(2);
  });
});
