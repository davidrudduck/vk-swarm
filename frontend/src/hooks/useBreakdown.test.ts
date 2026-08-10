import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import type { ReactNode } from 'react';
import { useBreakdownProposal, useBreakdownMutations } from './useBreakdown';
import type {
  TaskBreakdownProposal,
  TaskBreakdownProposalItem,
  Task,
  ProposalItemInput,
} from 'shared/types';

vi.mock('@/lib/api/breakdown');

const mockProposal: TaskBreakdownProposal = {
  id: 'proposal-1',
  task_id: 'task-1',
  status: 'draft',
  execution_process_id: null,
  error: null,
  created_at: new Date('2026-08-01'),
  updated_at: new Date('2026-08-01'),
};

const mockItems: TaskBreakdownProposalItem[] = [
  {
    id: 'item-1',
    proposal_id: 'proposal-1',
    title: 'Subtask 1',
    description: 'Description 1',
    sort_order: BigInt(0),
    depends_on_item_ids: '[]',
    created_at: new Date('2026-08-01'),
  },
  {
    id: 'item-2',
    proposal_id: 'proposal-1',
    title: 'Subtask 2',
    description: null,
    sort_order: BigInt(1),
    depends_on_item_ids: '["item-1"]',
    created_at: new Date('2026-08-01'),
  },
];

const mockTask = {
  id: 'task-1',
  project_id: 'project-1',
  title: 'Parent Task',
  description: null,
  status: 'todo',
  created_at: new Date('2026-08-01'),
  updated_at: new Date('2026-08-01'),
  parent_task_id: null,
  sort_order: BigInt(0),
  assignee_id: null,
  labels: [],
  is_archived: false,
} as unknown as Task;

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const wrapper = ({ children }: { children: ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
  return { wrapper, queryClient };
}

describe('useBreakdownProposal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches breakdown proposal with items on mount', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.get).mockResolvedValue({
      proposal: mockProposal,
      items: mockItems,
    });

    const { result } = renderHook(() => useBreakdownProposal('task-1'), {
      wrapper: createWrapper().wrapper,
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.proposal).toEqual(mockProposal);
    expect(result.current.items).toEqual(mockItems);
    expect(result.current.error).toBeNull();
    expect(vi.mocked(api.get)).toHaveBeenCalledWith('task-1');
  });

  it('handles null proposal (task with no breakdown)', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.get).mockResolvedValue(null);

    const { result } = renderHook(() => useBreakdownProposal('task-2'), {
      wrapper: createWrapper().wrapper,
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.proposal).toBeNull();
    expect(result.current.items).toEqual([]);
  });

  it('handles API errors', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    const error = new Error('Failed to fetch');
    vi.mocked(api.get).mockRejectedValue(error);

    const { result } = renderHook(() => useBreakdownProposal('task-1'), {
      wrapper: createWrapper().wrapper,
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.error).toEqual(error);
    expect(result.current.proposal).toBeNull();
    expect(result.current.items).toEqual([]);
  });
});

describe('useBreakdownMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('trigger mutation invalidates breakdown cache', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.trigger).mockResolvedValue(mockProposal);

    const { wrapper, queryClient } = createWrapper();
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(
      () => useBreakdownMutations('task-1', 'project-1'),
      { wrapper }
    );

    result.current.trigger.mutate(undefined);

    await waitFor(() => expect(result.current.trigger.isSuccess).toBe(true));

    expect(vi.mocked(api.trigger)).toHaveBeenCalledWith('task-1');
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['breakdown', 'task-1'],
    });
  });

  it('putItems mutation invalidates breakdown cache', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.putItems).mockResolvedValue(mockItems);

    const { wrapper, queryClient } = createWrapper();
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(
      () => useBreakdownMutations('task-1', 'project-1'),
      { wrapper }
    );

    const itemInputs: ProposalItemInput[] = [
      {
        title: 'Subtask 1',
        description: 'Description 1',
        sort_order: BigInt(0),
        depends_on_indices: [],
      },
      {
        title: 'Subtask 2',
        description: null,
        sort_order: BigInt(1),
        depends_on_indices: [BigInt(0)],
      },
    ];
    const payload = { items: itemInputs };
    result.current.putItems.mutate({ proposalId: 'proposal-1', payload });

    await waitFor(() => expect(result.current.putItems.isSuccess).toBe(true));

    expect(vi.mocked(api.putItems)).toHaveBeenCalledWith('proposal-1', payload);
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['breakdown', 'task-1'],
    });
  });

  it('discard mutation invalidates breakdown cache', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.discard).mockResolvedValue(mockProposal);

    const { wrapper, queryClient } = createWrapper();
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(
      () => useBreakdownMutations('task-1', 'project-1'),
      { wrapper }
    );

    result.current.discard.mutate('proposal-1');

    await waitFor(() => expect(result.current.discard.isSuccess).toBe(true));

    expect(vi.mocked(api.discard)).toHaveBeenCalledWith('proposal-1');
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['breakdown', 'task-1'],
    });
  });

  it('retry mutation invalidates breakdown cache', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.retry).mockResolvedValue(mockProposal);

    const { wrapper, queryClient } = createWrapper();
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(
      () => useBreakdownMutations('task-1', 'project-1'),
      { wrapper }
    );

    result.current.retry.mutate('proposal-1');

    await waitFor(() => expect(result.current.retry.isSuccess).toBe(true));

    expect(vi.mocked(api.retry)).toHaveBeenCalledWith('proposal-1');
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['breakdown', 'task-1'],
    });
  });

  it('accept mutation invalidates breakdown and tasks caches', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.accept).mockResolvedValue([mockTask]);

    const { wrapper, queryClient } = createWrapper();
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(
      () => useBreakdownMutations('task-1', 'project-1'),
      { wrapper }
    );

    result.current.accept.mutate('proposal-1');

    await waitFor(() => expect(result.current.accept.isSuccess).toBe(true));

    expect(vi.mocked(api.accept)).toHaveBeenCalledWith('proposal-1');
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['breakdown', 'task-1'],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ['tasks', 'project-1'],
    });
  });

  it('mutations call onSuccess callbacks', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.trigger).mockResolvedValue(mockProposal);

    const onTriggerSuccess = vi.fn();

    const { result } = renderHook(
      () =>
        useBreakdownMutations('task-1', 'project-1', {
          onTriggerSuccess,
        }),
      { wrapper: createWrapper().wrapper }
    );

    result.current.trigger.mutate(undefined);

    await waitFor(() => expect(result.current.trigger.isSuccess).toBe(true));

    expect(onTriggerSuccess).toHaveBeenCalledWith(mockProposal);
  });

  it('mutations call onError callbacks', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    const error = new Error('API failed');
    vi.mocked(api.trigger).mockRejectedValue(error);

    const onTriggerError = vi.fn();

    const { result } = renderHook(
      () =>
        useBreakdownMutations('task-1', 'project-1', {
          onTriggerError,
        }),
      { wrapper: createWrapper().wrapper }
    );

    result.current.trigger.mutate(undefined);

    await waitFor(() => expect(result.current.trigger.isError).toBe(true));

    expect(onTriggerError).toHaveBeenCalledWith(error);
  });

  /**
   * Only `trigger`'s error path was covered above, so a mutation whose onError
   * forgot to invoke its callback (or invoked the wrong one) stayed invisible.
   * Each mutation is exercised here through its own rejection.
   */
  describe('every mutation surfaces its own failure', () => {
    const cases = [
      {
        name: 'putItems',
        method: 'putItems',
        option: 'onPutItemsError',
        run: (m: ReturnType<typeof useBreakdownMutations>) =>
          m.putItems.mutate({
            proposalId: 'proposal-1',
            payload: { items: [] },
          }),
        pending: (m: ReturnType<typeof useBreakdownMutations>) => m.putItems,
      },
      {
        name: 'discard',
        method: 'discard',
        option: 'onDiscardError',
        run: (m: ReturnType<typeof useBreakdownMutations>) =>
          m.discard.mutate('proposal-1'),
        pending: (m: ReturnType<typeof useBreakdownMutations>) => m.discard,
      },
      {
        name: 'retry',
        method: 'retry',
        option: 'onRetryError',
        run: (m: ReturnType<typeof useBreakdownMutations>) =>
          m.retry.mutate('proposal-1'),
        pending: (m: ReturnType<typeof useBreakdownMutations>) => m.retry,
      },
      {
        name: 'accept',
        method: 'accept',
        option: 'onAcceptError',
        run: (m: ReturnType<typeof useBreakdownMutations>) =>
          m.accept.mutate('proposal-1'),
        pending: (m: ReturnType<typeof useBreakdownMutations>) => m.accept,
      },
    ] as const;

    it.each(cases)(
      '$name calls its onError callback with the rejection',
      async ({ method, option, run, pending }) => {
        const { breakdownApi: api } = await import('@/lib/api/breakdown');
        const error = new Error(`${method} failed`);
        vi.mocked(
          api[method] as unknown as (...args: unknown[]) => Promise<unknown>
        ).mockRejectedValue(error);
        vi.spyOn(console, 'error').mockImplementation(() => {});

        const onError = vi.fn();

        const { result } = renderHook(
          () =>
            useBreakdownMutations('task-1', 'project-1', { [option]: onError }),
          { wrapper: createWrapper().wrapper }
        );

        run(result.current);

        await waitFor(() => expect(pending(result.current).isError).toBe(true));

        expect(onError).toHaveBeenCalledWith(error);
      }
    );
  });
});

describe('useBreakdownProposal refetchInterval plumbing', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('passes a predicate through to react-query and polls on its verdict', async () => {
    // The predicate itself is unit-tested next to the dialog; this pins the
    // WIRING — that a function option reaches react-query in the shape its v5
    // callback expects (it receives the Query, not the data). A shape mismatch
    // here would silently disable polling with no type error at the call site.
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.get).mockResolvedValue({ proposal: mockProposal, items: [] });

    const { result } = renderHook(
      () =>
        useBreakdownProposal('task-1', {
          refetchInterval: (data) => (data?.items.length === 0 ? 10 : false),
        }),
      { wrapper: createWrapper().wrapper }
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    const afterFirst = vi.mocked(api.get).mock.calls.length;

    await waitFor(
      () =>
        expect(vi.mocked(api.get).mock.calls.length).toBeGreaterThan(
          afterFirst
        ),
      { timeout: 2000 }
    );
  });

  it('does not poll when the predicate returns false', async () => {
    const { breakdownApi: api } = await import('@/lib/api/breakdown');
    vi.mocked(api.get).mockResolvedValue({
      proposal: mockProposal,
      items: mockItems,
    });

    const { result } = renderHook(
      () => useBreakdownProposal('task-1', { refetchInterval: () => false }),
      { wrapper: createWrapper().wrapper }
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    const afterFirst = vi.mocked(api.get).mock.calls.length;

    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(vi.mocked(api.get).mock.calls.length).toBe(afterFirst);
  });
});
