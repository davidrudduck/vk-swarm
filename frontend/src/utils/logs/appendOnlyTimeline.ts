import type { PatchTypeWithKey } from '@/utils/logEntryToPatch';

const transientPatchKeys = new Set(['loading', 'next_action']);
const isTransientItem = (item: PatchTypeWithKey) =>
  transientPatchKeys.has(item.patchKey);

const serializeForRender = (value: unknown) =>
  JSON.stringify(value, (_key, itemValue) =>
    typeof itemValue === 'bigint' ? itemValue.toString() : itemValue
  );

export const getLogicalPatchKey = (patchKey: string) => patchKey;

const getItemRenderSignature = (item: PatchTypeWithKey) =>
  serializeForRender({
    ...item,
    patchKey: getLogicalPatchKey(item.patchKey),
  });

const getItemConversationSignature = (item: PatchTypeWithKey) =>
  serializeForRender({
    ...item,
    patchKey: undefined,
  });

const findInsertionIndex = (
  items: PatchTypeWithKey[],
  nextKeys: Set<string>,
  nextItems: PatchTypeWithKey[],
  newItemIndex: number,
  itemIndexes: Map<string, number>
) => {
  for (let index = newItemIndex + 1; index < nextItems.length; index += 1) {
    const anchorKey = nextItems[index]?.patchKey;
    if (!anchorKey) {
      continue;
    }

    const anchorIndex = itemIndexes.get(anchorKey);
    if (anchorIndex !== undefined) {
      let insertionIndex = anchorIndex;
      while (
        insertionIndex > 0 &&
        !nextKeys.has(items[insertionIndex - 1]?.patchKey ?? '')
      ) {
        insertionIndex -= 1;
      }
      return insertionIndex;
    }
  }

  return items.length;
};

const rebuildItemIndexes = (
  items: PatchTypeWithKey[],
  itemIndexes: Map<string, number>,
  startIndex = 0
) => {
  for (let index = startIndex; index < items.length; index += 1) {
    const item = items[index];
    if (item) {
      itemIndexes.set(item.patchKey, index);
    }
  }
};

export const mergeAppendOnlyItems = (
  previousItems: PatchTypeWithKey[],
  nextItems: PatchTypeWithKey[]
) => {
  const previousPersistentItems = previousItems.filter(
    (item) => !isTransientItem(item)
  );
  const nextPersistentItems = nextItems.filter(
    (item) => !isTransientItem(item)
  );
  const nextTransientItems = nextItems.filter((item) => isTransientItem(item));

  const nextKeys = new Set(nextPersistentItems.map((item) => item.patchKey));
  const includesAllPrevious = previousPersistentItems.every((item) =>
    nextKeys.has(item.patchKey)
  );

  if (includesAllPrevious) {
    return [...nextPersistentItems, ...nextTransientItems];
  }

  const mergedPersistentItems = [...previousPersistentItems];
  const mergedIndexes = new Map(
    mergedPersistentItems.map((item, index) => [item.patchKey, index])
  );

  nextPersistentItems.forEach((item, nextIndex) => {
    const existingIndex = mergedIndexes.get(item.patchKey);

    if (existingIndex !== undefined) {
      mergedPersistentItems[existingIndex] = item;
      return;
    }

    const insertionIndex = findInsertionIndex(
      mergedPersistentItems,
      nextKeys,
      nextPersistentItems,
      nextIndex,
      mergedIndexes
    );
    mergedPersistentItems.splice(insertionIndex, 0, item);
    rebuildItemIndexes(mergedPersistentItems, mergedIndexes, insertionIndex);
  });

  return [...mergedPersistentItems, ...nextTransientItems];
};

export const getTailRenderSignature = (items: PatchTypeWithKey[]) =>
  items
    .slice(-2)
    .map((item) => `${item.patchKey}:${getItemRenderSignature(item)}`)
    .join('|');

export const getAutoFollowTarget = (items: PatchTypeWithKey[]) => {
  if (items.length === 0) {
    return { index: 0, align: 'end' as const };
  }

  const lastItem = items[items.length - 1];
  if (!lastItem) {
    return { index: 0, align: 'end' as const };
  }

  if (!isTransientItem(lastItem)) {
    return {
      index: items.length - 1,
      align: 'end' as const,
    };
  }

  for (let index = items.length - 2; index >= 0; index -= 1) {
    const item = items[index];
    if (item && !isTransientItem(item)) {
      return {
        index,
        align: 'start' as const,
      };
    }
  }

  return {
    index: items.length - 1,
    align: 'end' as const,
  };
};

const getPersistentItems = (items: PatchTypeWithKey[]) =>
  items.filter((item) => !isTransientItem(item));

export const getRunningAppendOnlyResult = (
  previousItems: PatchTypeWithKey[],
  nextItems: PatchTypeWithKey[],
  _getNextRevision: (logicalPatchKey: string) => number,
  previousSnapshotItems: PatchTypeWithKey[] = previousItems
) => {
  const previousPersistentItems = getPersistentItems(previousItems);
  const previousSnapshotPersistentItems = getPersistentItems(
    previousSnapshotItems
  );
  const nextPersistentItems = getPersistentItems(nextItems);
  const nextTransientItems = nextItems.filter((item) => isTransientItem(item));
  const isObviousStaleReplay =
    nextPersistentItems.length < previousSnapshotPersistentItems.length &&
    nextPersistentItems.every((item, index) => {
      const previousSnapshotItem = previousSnapshotPersistentItems[index];
      return (
        !!previousSnapshotItem &&
        getItemConversationSignature(previousSnapshotItem) ===
          getItemConversationSignature(item)
      );
    });

  if (isObviousStaleReplay) {
    return {
      acceptedSnapshot: false,
      items: [...previousPersistentItems, ...nextTransientItems],
    };
  }

  return {
    acceptedSnapshot: true,
    items: [...nextPersistentItems, ...nextTransientItems],
  };
};

export const mergeRunningAppendOnlyItems = (
  previousItems: PatchTypeWithKey[],
  nextItems: PatchTypeWithKey[],
  getNextRevision: (logicalPatchKey: string) => number,
  previousSnapshotItems: PatchTypeWithKey[] = previousItems
) =>
  getRunningAppendOnlyResult(
    previousItems,
    nextItems,
    getNextRevision,
    previousSnapshotItems
  ).items;
