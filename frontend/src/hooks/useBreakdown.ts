import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { breakdownApi } from '@/lib/api/breakdown';
import type {
  TaskBreakdownProposal,
  TaskBreakdownProposalItem,
  UpsertProposalItems,
  Task,
} from 'shared/types';

/**
 * Hook options for useBreakdownProposal.
 */
export interface UseBreakdownProposalOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

/**
 * Hook state returned by useBreakdownProposal.
 */
export interface UseBreakdownProposalState {
  proposal: TaskBreakdownProposal | null;
  items: TaskBreakdownProposalItem[];
  isLoading: boolean;
  error: Error | null;
}

/**
 * Hook options for useBreakdownMutations.
 */
export interface UseBreakdownMutationsOptions {
  onTriggerSuccess?: (proposal: TaskBreakdownProposal) => void;
  onTriggerError?: (error: unknown) => void;
  onPutItemsSuccess?: (items: TaskBreakdownProposalItem[]) => void;
  onPutItemsError?: (error: unknown) => void;
  onDiscardSuccess?: (proposal: TaskBreakdownProposal) => void;
  onDiscardError?: (error: unknown) => void;
  onRetrySuccess?: (proposal: TaskBreakdownProposal) => void;
  onRetryError?: (error: unknown) => void;
  onAcceptSuccess?: (tasks: Task[]) => void;
  onAcceptError?: (error: unknown) => void;
}

/**
 * Stable empty-array reference so `items` keeps referential identity across
 * renders while the query has no data (a fresh `[]` each render caused an
 * unbounded effect loop in consumers that depend on `items` identity).
 */
const EMPTY_ITEMS: TaskBreakdownProposalItem[] = [];

/**
 * Fetch the latest breakdown proposal (with items) for a task.
 * Returns null if no proposal exists.
 */
export function useBreakdownProposal(
  taskId: string,
  options?: UseBreakdownProposalOptions
): UseBreakdownProposalState {
  const { data, isLoading, error } = useQuery({
    queryKey: ['breakdown', taskId],
    queryFn: () => breakdownApi.get(taskId),
    enabled: options?.enabled !== false,
    refetchInterval: options?.refetchInterval,
  });

  return {
    proposal: data?.proposal ?? null,
    items: data?.items ?? EMPTY_ITEMS,
    isLoading,
    error: error instanceof Error ? error : null,
  };
}

/**
 * Mutations for task breakdown operations: trigger, edit items, discard, retry, accept.
 */
export function useBreakdownMutations(
  taskId: string,
  projectId: string,
  options?: UseBreakdownMutationsOptions
) {
  const queryClient = useQueryClient();

  const invalidateBreakdown = () => {
    queryClient.invalidateQueries({ queryKey: ['breakdown', taskId] });
  };

  const trigger = useMutation({
    mutationFn: () => breakdownApi.trigger(taskId),
    onSuccess: (proposal) => {
      invalidateBreakdown();
      options?.onTriggerSuccess?.(proposal);
    },
    onError: (err) => {
      console.error('Failed to trigger breakdown:', err);
      options?.onTriggerError?.(err);
    },
  });

  const putItems = useMutation({
    mutationFn: ({
      proposalId,
      payload,
    }: {
      proposalId: string;
      payload: UpsertProposalItems;
    }) => breakdownApi.putItems(proposalId, payload),
    onSuccess: (items) => {
      invalidateBreakdown();
      options?.onPutItemsSuccess?.(items);
    },
    onError: (err) => {
      console.error('Failed to update proposal items:', err);
      options?.onPutItemsError?.(err);
    },
  });

  const discard = useMutation({
    mutationFn: (proposalId: string) => breakdownApi.discard(proposalId),
    onSuccess: (proposal) => {
      invalidateBreakdown();
      options?.onDiscardSuccess?.(proposal);
    },
    onError: (err) => {
      console.error('Failed to discard proposal:', err);
      options?.onDiscardError?.(err);
    },
  });

  const retry = useMutation({
    mutationFn: (proposalId: string) => breakdownApi.retry(proposalId),
    onSuccess: (proposal) => {
      invalidateBreakdown();
      options?.onRetrySuccess?.(proposal);
    },
    onError: (err) => {
      console.error('Failed to retry proposal:', err);
      options?.onRetryError?.(err);
    },
  });

  const accept = useMutation({
    mutationFn: (proposalId: string) => breakdownApi.accept(proposalId),
    onSuccess: (tasks) => {
      invalidateBreakdown();
      queryClient.invalidateQueries({ queryKey: ['tasks', projectId] });
      options?.onAcceptSuccess?.(tasks);
    },
    onError: (err) => {
      console.error('Failed to accept proposal:', err);
      options?.onAcceptError?.(err);
    },
  });

  return {
    trigger,
    putItems,
    discard,
    retry,
    accept,
  };
}
