import { VList, VListHandle } from 'virtua';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import DisplayConversationEntry from '../NormalizedConversation/DisplayConversationEntry';
import { useEntries } from '@/contexts/EntriesContext';
import {
  AddEntryType,
  PatchTypeWithKey,
  useConversationHistory,
} from '@/hooks/useConversationHistory';
import { ArrowDown, ArrowUp, Loader2, Settings2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from '@/components/ui/select';
import {
  PaginationPreset,
  usePaginationOverride,
} from '@/stores/usePaginationOverride';
import { TaskAttempt, TaskWithAttemptStatus } from 'shared/types';
import { ApprovalFormProvider } from '@/contexts/ApprovalFormContext';
import { useTranslation } from 'react-i18next';

interface VirtualizedListProps {
  attempt: TaskAttempt;
  task?: TaskWithAttemptStatus;
}

interface MessageListContext {
  attempt: TaskAttempt;
  task?: TaskWithAttemptStatus;
}

const TRANSIENT_PATCH_KEYS = new Set(['loading', 'next_action']);
const APPEND_ONLY_REVISION_MARKER = '::append:';

const isTransientItem = (item: PatchTypeWithKey) =>
  TRANSIENT_PATCH_KEYS.has(item.patchKey);

const serializeForRender = (value: unknown) =>
  JSON.stringify(value, (_key, itemValue) =>
    typeof itemValue === 'bigint' ? itemValue.toString() : itemValue
  );

const getLogicalPatchKey = (patchKey: string) => {
  const markerIndex = patchKey.indexOf(APPEND_ONLY_REVISION_MARKER);
  return markerIndex === -1 ? patchKey : patchKey.slice(0, markerIndex);
};

const getItemRenderSignature = (item: PatchTypeWithKey) =>
  serializeForRender({
    ...item,
    patchKey: getLogicalPatchKey(item.patchKey),
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
  const nextPersistentItems = nextItems.filter((item) => !isTransientItem(item));
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

const isTextualAppendOnlyAdvance = (
  previousItem: PatchTypeWithKey,
  nextItem: PatchTypeWithKey
) => {
  if (getLogicalPatchKey(previousItem.patchKey) !== getLogicalPatchKey(nextItem.patchKey)) {
    return false;
  }

  if (getItemRenderSignature(previousItem) === getItemRenderSignature(nextItem)) {
    return true;
  }

  if (
    (previousItem.type === 'STDOUT' || previousItem.type === 'STDERR') &&
    previousItem.type === nextItem.type
  ) {
    return nextItem.content.startsWith(previousItem.content);
  }

  if (
    previousItem.type === 'NORMALIZED_ENTRY' &&
    nextItem.type === 'NORMALIZED_ENTRY' &&
    previousItem.content.entry_type.type === nextItem.content.entry_type.type
  ) {
    return nextItem.content.content.startsWith(previousItem.content.content);
  }

  return false;
};

export const isRunningSnapshotReplay = (
  previousItems: PatchTypeWithKey[],
  nextItems: PatchTypeWithKey[]
) => {
  const previousPersistentItems = getPersistentItems(previousItems);
  const nextPersistentItems = getPersistentItems(nextItems);

  if (previousPersistentItems.length === 0 || nextPersistentItems.length === 0) {
    return false;
  }

  if (nextPersistentItems.length < previousPersistentItems.length) {
    return true;
  }

  const commonLength = previousPersistentItems.length;
  for (let index = 0; index < commonLength; index += 1) {
    const previousItem = previousPersistentItems[index]!;
    const nextItem = nextPersistentItems[index]!;

    if (getLogicalPatchKey(previousItem.patchKey) !== getLogicalPatchKey(nextItem.patchKey)) {
      return true;
    }

    const isLastSharedItem = index === commonLength - 1;
    if (isLastSharedItem) {
      continue;
    }

    if (getItemRenderSignature(previousItem) !== getItemRenderSignature(nextItem)) {
      return true;
    }
  }

  return !isTextualAppendOnlyAdvance(
    previousPersistentItems[commonLength - 1]!,
    nextPersistentItems[commonLength - 1]!
  );
};

export const mergeRunningAppendOnlyItems = (
  previousItems: PatchTypeWithKey[],
  nextItems: PatchTypeWithKey[],
  getNextRevision: (logicalPatchKey: string) => number
) => {
  const previousPersistentItems = getPersistentItems(previousItems);
  const nextPersistentItems = getPersistentItems(nextItems);
  const nextTransientItems = nextItems.filter((item) => isTransientItem(item));
  const mergedItems = [...previousPersistentItems];

  nextPersistentItems.forEach((item) => {
    const logicalPatchKey = getLogicalPatchKey(item.patchKey);
    const lastMatchingItem = [...mergedItems]
      .reverse()
      .find(
        (existingItem) => getLogicalPatchKey(existingItem.patchKey) === logicalPatchKey
      );

    if (!lastMatchingItem) {
      mergedItems.push(item);
      return;
    }

    if (getItemRenderSignature(lastMatchingItem) === getItemRenderSignature(item)) {
      return;
    }

    mergedItems.push({
      ...item,
      patchKey: `${logicalPatchKey}${APPEND_ONLY_REVISION_MARKER}${getNextRevision(
        logicalPatchKey
      )}`,
    });
  });

  return [...mergedItems, ...nextTransientItems];
};

const ItemContent = ({
  data,
  context,
}: {
  data: PatchTypeWithKey;
  context?: MessageListContext;
}) => {
  const attempt = context?.attempt;
  const task = context?.task;

  if (data.type === 'STDOUT') {
    return <p>{data.content}</p>;
  }
  if (data.type === 'STDERR') {
    return <p>{data.content}</p>;
  }
  if (data.type === 'NORMALIZED_ENTRY' && attempt) {
    return (
      <DisplayConversationEntry
        expansionKey={data.patchKey}
        entry={data.content}
        executionProcessId={data.executionProcessId}
        taskAttempt={attempt}
        task={task}
      />
    );
  }

  return null;
};

const VirtualizedList = ({ attempt, task }: VirtualizedListProps) => {
  const { t } = useTranslation('common');
  const [items, setItems] = useState<PatchTypeWithKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [atBottom, setAtBottom] = useState(true);
  const [atTop, setAtTop] = useState(true);
  const { setEntries, reset } = useEntries();
  const [paginationOverride, setPaginationOverride] = usePaginationOverride(
    attempt.id
  );
  const itemsRef = useRef<PatchTypeWithKey[]>([]);
  const listRef = useRef<VListHandle>(null);
  const previousTailSignatureRef = useRef('');
  const appendOnlyRevisionRef = useRef<Record<string, number>>({});
  const runningSnapshotRef = useRef<PatchTypeWithKey[]>([]);
  const runningReplaySnapshotRef = useRef<PatchTypeWithKey[] | null>(null);

  useEffect(() => {
    setLoading(true);
    setItems([]);
    itemsRef.current = [];
    previousTailSignatureRef.current = '';
    appendOnlyRevisionRef.current = {};
    runningSnapshotRef.current = [];
    runningReplaySnapshotRef.current = null;
    reset();
    didInitScroll.current = false;
    prevLenRef.current = 0;
    setAtTop(true);
    setAtBottom(true);
  }, [attempt.id, reset]);

  const onEntriesUpdated = (
    newEntries: PatchTypeWithKey[],
    addType: AddEntryType,
    newLoading: boolean
  ) => {
    const nextRunningRevision = (logicalPatchKey: string) => {
      const nextRevision = (appendOnlyRevisionRef.current[logicalPatchKey] ?? 0) + 1;
      appendOnlyRevisionRef.current[logicalPatchKey] = nextRevision;
      return nextRevision;
    };

    let mergedItems: PatchTypeWithKey[];
    if (addType === 'running') {
      const shouldSuppressAsReplay = isRunningSnapshotReplay(
        runningSnapshotRef.current,
        newEntries
      );

      if (runningReplaySnapshotRef.current || shouldSuppressAsReplay) {
        if (shouldSuppressAsReplay) {
          runningReplaySnapshotRef.current = newEntries;
          mergedItems = [
            ...getPersistentItems(itemsRef.current),
            ...newEntries.filter(isTransientItem),
          ];
        } else {
          runningReplaySnapshotRef.current = null;
          mergedItems = mergeRunningAppendOnlyItems(
            itemsRef.current,
            newEntries,
            nextRunningRevision
          );
          runningSnapshotRef.current = newEntries;
        }
      } else {
        mergedItems = mergeRunningAppendOnlyItems(
          itemsRef.current,
          newEntries,
          nextRunningRevision
        );
        runningSnapshotRef.current = newEntries;
      }
    } else {
      mergedItems = mergeAppendOnlyItems(itemsRef.current, newEntries);
      runningSnapshotRef.current = newEntries;
      runningReplaySnapshotRef.current = null;
    }

    itemsRef.current = mergedItems;
    setItems(mergedItems);
    setEntries(mergedItems);

    if (loading) {
      setLoading(newLoading);
    }
  };

  useConversationHistory({ attempt, onEntriesUpdated });

  const didInitScroll = useRef(false);
  const prevLenRef = useRef(0);
  const messageListContext = useMemo(
    () => ({ attempt, task }),
    [attempt, task]
  );

  const scrollToBottom = useCallback(() => {
    listRef.current?.scrollToIndex(items.length - 1, {
      align: 'end',
      smooth: false,
    });
    requestAnimationFrame(() => setAtBottom(true));
  }, [items.length]);

  const scrollToTop = useCallback(() => {
    listRef.current?.scrollToIndex(0, { align: 'start', smooth: true });
  }, []);

  // Initial jump to bottom + auto-follow during streaming
  useEffect(() => {
    const prev = prevLenRef.current;
    const grewBy = items.length - prev;
    const previousTailSignature = previousTailSignatureRef.current;
    const nextTailSignature = getTailRenderSignature(items);
    prevLenRef.current = items.length;
    previousTailSignatureRef.current = nextTailSignature;

    if (items.length === 0) return;

    if (!didInitScroll.current) {
      didInitScroll.current = true;
      // Double rAF: first frame lets virtua render, second lets it measure item heights.
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          listRef.current?.scrollToIndex(items.length - 1, { align: 'end' });
        });
      });
      return;
    }

    const tailAdvanced =
      nextTailSignature.length > 0 &&
      nextTailSignature !== previousTailSignature;

    if ((grewBy > 0 || tailAdvanced) && atBottom && !loading) {
      requestAnimationFrame(scrollToBottom);
    }
  }, [items, atBottom, loading, scrollToBottom]);

  return (
    <ApprovalFormProvider>
      <div className="relative flex-1 flex flex-col">
        <VList
          ref={listRef}
          className="flex-1"
          data={items}
          bufferSize={600}
          style={{ paddingTop: 8, paddingBottom: 8 }}
          onScroll={(offset) => {
            const handle = listRef.current;
            if (!handle) return;
            setAtTop(offset <= 0);
            setAtBottom(offset + handle.viewportSize >= handle.scrollSize - 20);
          }}
        >
          {(item) => (
            <ItemContent
              key={item.patchKey}
              data={item}
              context={messageListContext}
            />
          )}
        </VList>
        {!atTop && items.length > 0 && !loading && (
          <Button
            variant="outline"
            size="icon"
            className="absolute top-4 right-4 rounded-full shadow-lg bg-background/90 backdrop-blur-sm hover:bg-background z-10"
            onClick={scrollToTop}
            aria-label="Scroll to top"
          >
            <ArrowUp className="h-4 w-4" />
          </Button>
        )}
        {!loading && items.length > 0 && (
          <div className="absolute top-1/2 -translate-y-1/2 right-4 z-10">
            <Select
              value={String(paginationOverride)}
              onValueChange={(value) =>
                setPaginationOverride(
                  value === 'global'
                    ? 'global'
                    : (Number(value) as PaginationPreset)
                )
              }
            >
              <SelectTrigger
                className="w-8 h-8 p-0 justify-center rounded-full shadow-lg bg-background/90 backdrop-blur-sm border-input [&>svg:last-child]:hidden"
                aria-label="Pagination settings"
              >
                <Settings2 className="h-4 w-4" />
              </SelectTrigger>
              <SelectContent align="end">
                <SelectItem value="global">
                  {t('conversation.pagination.global')}
                </SelectItem>
                <SelectItem value="50">
                  {t('conversation.pagination.entries50')}
                </SelectItem>
                <SelectItem value="100">
                  {t('conversation.pagination.entries100')}
                </SelectItem>
                <SelectItem value="200">
                  {t('conversation.pagination.entries200')}
                </SelectItem>
                <SelectItem value="500">
                  {t('conversation.pagination.entries500')}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        )}
        {!atBottom && items.length > 0 && !loading && (
          <Button
            variant="outline"
            size="icon"
            className="absolute bottom-4 right-4 rounded-full shadow-lg bg-background/90 backdrop-blur-sm hover:bg-background z-10"
            onClick={scrollToBottom}
            aria-label="Scroll to bottom"
          >
            <ArrowDown className="h-4 w-4" />
          </Button>
        )}
      </div>
      {loading && (
        <div className="float-left top-0 left-0 w-full h-full bg-primary flex flex-col gap-2 justify-center items-center">
          <Loader2 className="h-8 w-8 animate-spin" />
          <p>{t('conversation.loadingHistory')}</p>
        </div>
      )}
    </ApprovalFormProvider>
  );
};

export default VirtualizedList;
