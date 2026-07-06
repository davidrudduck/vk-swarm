import { get, update } from 'idb-keyval';

export interface MutationEntry {
  id: string;
  operation: string;
  endpoint: string;
  payload: unknown;
  timestamp: number;
}

const QUEUE_KEY = 'offline-mutation-queue';

export async function enqueueMutation(
  operation: string,
  endpoint: string,
  payload: unknown,
): Promise<void> {
  await update<MutationEntry[]>(QUEUE_KEY, (queue = []) => [
    ...queue,
    {
      id: crypto.randomUUID(),
      operation,
      endpoint,
      payload,
      timestamp: Date.now(),
    },
  ]);
}

export async function replayMutations(
  execute: (entry: MutationEntry) => Promise<void>,
  onError: (entry: MutationEntry, error: Error) => void,
): Promise<void> {
  let queue: MutationEntry[] = [];

  await update<MutationEntry[]>(QUEUE_KEY, (current = []) => {
    queue = current;
    return [];
  });

  if (queue.length === 0) return;

  const remaining: MutationEntry[] = [];

  for (const entry of queue) {
    try {
      await execute(entry);
    } catch (err) {
      onError(entry, err instanceof Error ? err : new Error(String(err)));
      remaining.push(entry);
    }
  }

  await update<MutationEntry[]>(QUEUE_KEY, (current = []) => [
    ...remaining,
    ...current,
  ]);
}

export async function getQueueLength(): Promise<number> {
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  return queue?.length ?? 0;
}
