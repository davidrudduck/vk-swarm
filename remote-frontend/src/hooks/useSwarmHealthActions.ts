import { useState, useCallback } from 'react';

export interface FixAllResult {
  successCount: number;
  errorCount: number;
  errors: Array<{ projectId: string; projectName: string; error: unknown }>;
}

interface UseSwarmHealthActionsOptions {
  onFixAllSuccess?: (result: FixAllResult) => void;
  onFixAllError?: (error: unknown) => void;
  onFixAllPartial?: (result: FixAllResult) => void;
}

export function useSwarmHealthActions(options?: UseSwarmHealthActionsOptions) {
  const [isFixing, setIsFixing] = useState(false);

  const fixAllIssues = useCallback(async (): Promise<FixAllResult> => {
    setIsFixing(true);
    try {
      const result: FixAllResult = {
        successCount: 0,
        errorCount: 0,
        errors: [],
      };
      options?.onFixAllSuccess?.(result);
      return result;
    } finally {
      setIsFixing(false);
    }
  }, [options]);

  return { fixAllIssues, isFixing };
}