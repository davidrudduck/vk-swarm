import type { PatchTypeWithKey } from '@/hooks/useConversationHistory';

const TRANSIENT_PATCH_KEYS = new Set(['loading', 'next_action']);
const APPEND_ONLY_REVISION_MARKER = '::append:';

const isTransientItem = (item: PatchTypeWithKey) =>
  TRANSIENT_PATCH_KEYS.has(item.patchKey);

const serializeForRender = (value: unknown) =>
  JSON.stringify(value, (_key, itemValue) =>
    typeof itemValue === 'bigint' ? itemValue.toString() : itemValue
  );

export const getLogicalPatchKey = (patchKey: string) => {
  const markerIndex = patchKey.indexOf(APPEND_ONLY_REVISION_MARKER);
  return markerIndex === -1 ? patchKey : patchKey.slice(0, markerIndex);
};

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
  newItemIndex: number
) => {
  for (let index = newItemIndex + 1; index < nextItems.length; index += 1) {
    const anchorKey = nextItems[index]?.patchKey;
    const anchorIndex = items.findIndex((item) => item.patchKey === anchorKey);
    if (anchorIndex !== -1) {
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

  nextPersistentItems.forEach((item, nextIndex) => {
    const existingIndex = mergedPersistentItems.findIndex(
      (existingItem) => existingItem.patchKey === item.patchKey
    );

    if (existingIndex !== -1) {
      mergedPersistentItems[existingIndex] = item;
      return;
    }

    const insertionIndex = findInsertionIndex(
      mergedPersistentItems,
      nextKeys,
      nextPersistentItems,
      nextIndex
    );
    mergedPersistentItems.splice(insertionIndex, 0, item);
  });

  return [...mergedPersistentItems, ...nextTransientItems];
};

export const getTailRenderSignature = (items: PatchTypeWithKey[]) =>
  items
    .slice(-2)
    .map((item) => `${item.patchKey}:${getItemRenderSignature(item)}`)
    .join('|');

const getPersistentItems = (items: PatchTypeWithKey[]) =>
  items.filter((item) => !isTransientItem(item));

const getCommandRunAppendOnlyText = (item: PatchTypeWithKey) => {
  if (item.type !== 'NORMALIZED_ENTRY') {
    return null;
  }

  const entryType = item.content.entry_type;
  if (
    entryType.type !== 'tool_use' ||
    entryType.action_type.action !== 'command_run'
  ) {
    return null;
  }

  const result = entryType.action_type.result;
  const segments = [
    `tool:${entryType.tool_name}`,
    `label:${item.content.content}`,
    `command:${entryType.action_type.command}`,
    `output:${result?.output ?? ''}`,
  ];

  if (result?.exit_status) {
    segments.push(`exit:${serializeForRender(result.exit_status)}`);
  }

  if (entryType.status.status !== 'created') {
    segments.push(`status:${serializeForRender(entryType.status)}`);
  }

  return segments.join('\n');
};

const getNormalizedTextAppendOnlyText = (item: PatchTypeWithKey) => {
  if (item.type !== 'NORMALIZED_ENTRY') {
    return null;
  }

  const entryType = item.content.entry_type;
  switch (entryType.type) {
    case 'assistant_message':
    case 'thinking':
      return `type:${entryType.type}\ncontent:${item.content.content}`;
    default:
      return null;
  }
};

export const getRunningAppendOnlyResult = (
  previousItems: PatchTypeWithKey[],
  nextItems: PatchTypeWithKey[],
  getNextRevision: (logicalPatchKey: string) => number,
  previousSnapshotItems: PatchTypeWithKey[] = previousItems
) => {
  const previousPersistentItems = getPersistentItems(previousItems);
  const previousSnapshotPersistentItems = getPersistentItems(
    previousSnapshotItems
  );
  const nextPersistentItems = getPersistentItems(nextItems);
  const nextTransientItems = nextItems.filter((item) => isTransientItem(item));
  const mergedItems = [...previousPersistentItems];
  let previousSnapshotIndex = 0;
  const pendingAppends: Array<{
    item: PatchTypeWithKey;
    priorPatchKey: string | null;
  }> = [];

  nextPersistentItems.forEach((item) => {
    const previousSnapshotItem =
      previousSnapshotPersistentItems[previousSnapshotIndex];
    const isExactMatch =
      previousSnapshotItem &&
      getItemConversationSignature(previousSnapshotItem) ===
        getItemConversationSignature(item);
    const isStdoutAppendOnlyGrowth =
      !!previousSnapshotItem &&
      (previousSnapshotItem.type === 'STDOUT' ||
        previousSnapshotItem.type === 'STDERR') &&
      previousSnapshotItem.type === item.type &&
      item.content.startsWith(previousSnapshotItem.content);
    const previousCommandRunText = previousSnapshotItem
      ? getCommandRunAppendOnlyText(previousSnapshotItem)
      : null;
    const nextCommandRunText = getCommandRunAppendOnlyText(item);
    const previousNormalizedText = previousSnapshotItem
      ? getNormalizedTextAppendOnlyText(previousSnapshotItem)
      : null;
    const nextNormalizedText = getNormalizedTextAppendOnlyText(item);
    const isAppendOnlyGrowth =
      isStdoutAppendOnlyGrowth ||
      (!!previousSnapshotItem &&
        !!previousCommandRunText &&
        !!nextCommandRunText &&
        nextCommandRunText.startsWith(previousCommandRunText)) ||
      (!!previousSnapshotItem &&
        !!previousNormalizedText &&
        !!nextNormalizedText &&
        nextNormalizedText.startsWith(previousNormalizedText));

    if (isExactMatch) {
      previousSnapshotIndex += 1;
      return;
    }

    if (isAppendOnlyGrowth) {
      pendingAppends.push({
        item,
        priorPatchKey: previousSnapshotItem.patchKey,
      });
      previousSnapshotIndex += 1;
      return;
    }

    pendingAppends.push({
      item,
      priorPatchKey: null,
    });
  });

  if (previousSnapshotIndex < previousSnapshotPersistentItems.length) {
    return {
      acceptedSnapshot: false,
      items: [...previousPersistentItems, ...nextTransientItems],
    };
  }

  pendingAppends.forEach(({ item, priorPatchKey }) => {
    const logicalPatchKey = getLogicalPatchKey(priorPatchKey ?? item.patchKey);
    const hasExistingLogicalPatchKey =
      priorPatchKey !== null ||
      mergedItems.some(
        (existingItem) =>
          getLogicalPatchKey(existingItem.patchKey) === logicalPatchKey
      );

    if (!hasExistingLogicalPatchKey) {
      mergedItems.push(item);
      return;
    }

    mergedItems.push({
      ...item,
      patchKey: `${logicalPatchKey}${APPEND_ONLY_REVISION_MARKER}${getNextRevision(
        logicalPatchKey
      )}`,
    });
  });

  return {
    acceptedSnapshot: true,
    items: [...mergedItems, ...nextTransientItems],
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
