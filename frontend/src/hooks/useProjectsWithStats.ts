import { useQuery } from '@tanstack/react-query';
import { projectsApi } from '@/lib/api';
import type { ProjectsWithStatsResponse } from 'shared/types';

/**
 * Hook to fetch this node's local projects with display enrichment
 * (task counts, last attempt, GitHub counts).
 */
export function useProjectsWithStats() {
  return useQuery<ProjectsWithStatsResponse>({
    queryKey: ['projects-with-stats'],
    queryFn: () => projectsApi.getWithStats(),
    staleTime: 30000,
  });
}
