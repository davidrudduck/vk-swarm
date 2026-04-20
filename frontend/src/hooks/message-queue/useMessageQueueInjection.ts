import { useCallback, useState } from 'react';
import { executionProcessesApi } from '@/lib/api';
import { useMessageQueue } from './useMessageQueue';

/**
 * Extended message queue hook that keeps queueing and live injection separate.
 *
 * Queueing and injection are intentionally separate actions:
 * 1. `queueMessage` persists a follow-up for the next turn
 * 2. `injectOnly` sends a one-off live message into the running process
 *
 * @param attemptId - The task attempt ID for the message queue
 * @param runningProcessId - The ID of the currently running execution process (if any)
 */
export function useMessageQueueInjection(
  attemptId: string | undefined,
  runningProcessId: string | undefined
) {
  const messageQueue = useMessageQueue(attemptId);
  const { addMessage } = messageQueue;
  const [isInjecting, setIsInjecting] = useState(false);
  const [lastInjectionError, setLastInjectionError] = useState<Error | null>(
    null
  );

  const queueMessage = useCallback(
    async (
      content: string,
      variant: string | null = null
    ): Promise<{ queued: boolean; injected: boolean }> => {
      await addMessage(content, variant);
      return { queued: true, injected: false };
    },
    [addMessage]
  );

  /**
   * Inject a message into the running process without adding to queue.
   * Use this when you want to send a one-off message that shouldn't persist.
   *
   * @param content - The message content
   * @returns Object with `injected: boolean`
   */
  const injectOnly = useCallback(
    async (content: string): Promise<{ injected: boolean }> => {
      if (!runningProcessId) {
        return { injected: false };
      }

      setIsInjecting(true);
      setLastInjectionError(null);

      try {
        const result = await executionProcessesApi.injectMessage(
          runningProcessId,
          content
        );
        return result;
      } catch (error) {
        console.error('Failed to inject message:', error);
        setLastInjectionError(
          error instanceof Error ? error : new Error(String(error))
        );
        return { injected: false };
      } finally {
        setIsInjecting(false);
      }
    },
    [runningProcessId]
  );

  const addAndInject = useCallback(
    async (
      content: string,
      variant: string | null = null
    ): Promise<{ queued: boolean; injected: boolean }> => {
      return queueMessage(content, variant);
    },
    [queueMessage]
  );

  return {
    // All properties from useMessageQueue
    ...messageQueue,

    // Queue and injection are exposed separately.
    queueMessage,
    addAndInject,
    injectOnly,
    isInjecting,
    lastInjectionError,
    canInject: !!runningProcessId,
  };
}
