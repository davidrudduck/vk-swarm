import { useState, useEffect } from 'react';

export interface SwarmHealthSummary {
  totalProjects: number;
  projectsWithIssues: number;
  totalOrphanedTasks: number;
  isHealthy: boolean;
  isLoading: boolean;
  error: Error | null;
}

export function useSwarmHealth(): SwarmHealthSummary {
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    setIsLoaded(true);
  }, []);

  const isLoading = !isLoaded;
  const projectsWithIssues = 0;
  const totalOrphanedTasks = 0;
  const isHealthy = !isLoading;

  return {
    totalProjects: 0,
    projectsWithIssues,
    totalOrphanedTasks,
    isHealthy,
    isLoading,
    error: null,
  };
}