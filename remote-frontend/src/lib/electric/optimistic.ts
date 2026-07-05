import type { QueryClient } from '@tanstack/react-query';

type CollectionItem = { id: string; [key: string]: unknown };

export async function optimisticDelete(
  queryClient: QueryClient,
  queryKey: string[],
  itemId: string,
  apiCall: () => Promise<unknown>,
): Promise<void> {
  const previous = queryClient.getQueryData<CollectionItem[]>(queryKey);
  queryClient.setQueryData<CollectionItem[]>(queryKey, (old) =>
    (old ?? []).filter((item) => item.id !== itemId),
  );

  try {
    await apiCall();
  } catch (err) {
    if (previous) {
      queryClient.setQueryData(queryKey, previous);
    }
    throw err;
  }
}

export async function optimisticUpdate(
  queryClient: QueryClient,
  queryKey: string[],
  itemId: string,
  patch: Record<string, unknown>,
  apiCall: () => Promise<unknown>,
): Promise<void> {
  const previous = queryClient.getQueryData<CollectionItem[]>(queryKey);
  queryClient.setQueryData<CollectionItem[]>(queryKey, (old) =>
    (old ?? []).map((item) =>
      item.id === itemId ? { ...item, ...patch } : item,
    ),
  );

  try {
    await apiCall();
  } catch (err) {
    if (previous) {
      queryClient.setQueryData(queryKey, previous);
    }
    throw err;
  }
}
