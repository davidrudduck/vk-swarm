import { describe, it, expect, vi, beforeEach } from 'vitest';
import { enqueueMutation, getQueueLength, replayMutations } from './mutation-queue';

vi.mock('idb-keyval', () => {
  const store: Record<string, unknown> = {};
  return {
    get: vi.fn(async (key: string) => store[key] ?? null),
    set: vi.fn(async (key: string, value: unknown) => { store[key] = value; }),
    update: vi.fn(async (key: string, updater: (old: unknown) => unknown) => {
      store[key] = updater(store[key]);
    }),
    del: vi.fn(async (key: string) => { delete store[key]; }),
  };
});
import { get, set, update } from 'idb-keyval';

describe('mutation queue module (SC10)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('enqueueMutation stores entry via atomic update', async () => {
    await enqueueMutation('DELETE', '/v1/tasks/t1', 't1');
    expect(update).toHaveBeenCalledWith('offline-mutation-queue', expect.any(Function));
  });

  it('getQueueLength returns 0 for empty queue', async () => {
    vi.mocked(get).mockResolvedValue(null);
    const length = await getQueueLength();
    expect(length).toBe(0);
  });

  it('getQueueLength returns count of queued entries', async () => {
    vi.mocked(get).mockResolvedValue([{ id: '1' }, { id: '2' }]);
    const length = await getQueueLength();
    expect(length).toBe(2);
  });

  it('replayMutations replays entries and removes successful ones', async () => {
    const entries = [
      { id: 'm1', operation: 'DELETE', endpoint: '/v1/tasks/t1', payload: 't1', timestamp: 1 },
      { id: 'm2', operation: 'PATCH', endpoint: '/v1/tasks/t2', payload: { taskId: 't2', nodeId: 'n1' }, timestamp: 2 },
    ];
    vi.mocked(get).mockResolvedValue(entries);
    const execute = vi.fn().mockResolvedValue(undefined);
    const onError = vi.fn();
    await replayMutations(execute, onError);
    expect(execute).toHaveBeenCalledTimes(2);
    expect(execute).toHaveBeenCalledWith(entries[0]);
    expect(execute).toHaveBeenCalledWith(entries[1]);
    expect(onError).not.toHaveBeenCalled();
  });

  it('replayMutations keeps failing entries and calls onError', async () => {
    const entries = [
      { id: 'm1', operation: 'DELETE', endpoint: '/v1/tasks/t1', payload: 't1', timestamp: 1 },
    ];
    vi.mocked(get).mockResolvedValue(entries);
    const execute = vi.fn().mockRejectedValue(new Error('network error'));
    const onError = vi.fn();
    await replayMutations(execute, onError);
    expect(onError).toHaveBeenCalledWith(entries[0], expect.any(Error));
    expect(set).toHaveBeenCalledWith('offline-mutation-queue', entries);
  });
});