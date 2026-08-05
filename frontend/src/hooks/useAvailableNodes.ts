import { useQuery } from '@tanstack/react-query';
import { tasksApi, type ListProjectNodesResponse } from '@/lib/api';
import { isHiveNotConfigured } from '@/lib/api/utils';

/**
 * Hook to fetch nodes where a task's project exists.
 * Used for selecting a node when starting a remote task attempt.
 *
 * When the node has no hive configured, the server responds with a 503
 * (`isHiveNotConfigured`). That is treated as a quiet "no nodes available"
 * outcome rather than a thrown error, so callers (e.g. `CreateAttemptDialog`)
 * can render their local-attempt path without touching error state or
 * triggering TanStack Query's default retry loop.
 */
export function useAvailableNodes(
  taskId: string | undefined,
  options?: { enabled?: boolean }
) {
  return useQuery<ListProjectNodesResponse>({
    queryKey: ['availableNodes', taskId],
    queryFn: async () => {
      try {
        return await tasksApi.availableNodes(taskId!);
      } catch (e) {
        if (isHiveNotConfigured(e)) {
          return { nodes: [] };
        }
        throw e;
      }
    },
    enabled: options?.enabled !== false && !!taskId,
    staleTime: 30000, // 30 seconds
  });
}
