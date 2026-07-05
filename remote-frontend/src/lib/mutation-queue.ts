import { get, set } from 'idb-keyval';

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
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  const entry: MutationEntry = {
    id: crypto.randomUUID(),
    operation,
    endpoint,
    payload,
    timestamp: Date.now(),
  };
  const updated = queue ? [...queue, entry] : [entry];
  await set(QUEUE_KEY, updated);
}

export async function replayMutations(
  execute: (entry: MutationEntry) => Promise<void>,
  onError: (entry: MutationEntry, error: Error) => void,
): Promise<void> {
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  if (!queue || queue.length === 0) return;

  const remaining: MutationEntry[] = [];

  for (const entry of queue) {
    try {
      await execute(entry);
    } catch (err) {
      onError(entry, err instanceof Error ? err : new Error(String(err)));
      remaining.push(entry);
    }
  }

  await set(QUEUE_KEY, remaining);
}

export async function getQueueLength(): Promise<number> {
  const queue = await get<MutationEntry[]>(QUEUE_KEY);
  return queue?.length ?? 0;
}
