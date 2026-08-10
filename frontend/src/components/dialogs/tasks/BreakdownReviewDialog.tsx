import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { ArrowDown, ArrowUp, Loader2, Trash2 } from 'lucide-react';

import { defineModal } from '@/lib/modals';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Checkbox } from '@/components/ui/checkbox';
import { Alert } from '@/components/ui/alert';
import {
  useBreakdownProposal,
  useBreakdownMutations,
} from '@/hooks/useBreakdown';
import type { BreakdownWithItems } from '@/lib/api/breakdown';
import type {
  TaskBreakdownProposalItem,
  ProposalItemInput,
} from 'shared/types';

export interface BreakdownReviewDialogProps {
  taskId: string;
  projectId: string;
}

/** Local, editable representation of a proposal item. Dependencies are
 * tracked by stable key (the original item id) rather than by array index,
 * so reorder/delete never need to walk existing dependency lists — the
 * index-based payload is only computed at save time, from final array order. */
interface LocalItem {
  key: string;
  title: string;
  description: string;
  dependsOn: Set<string>;
}

function toLocalItems(items: TaskBreakdownProposalItem[]): LocalItem[] {
  const sorted = [...items].sort((a, b) => Number(a.sort_order - b.sort_order));
  return sorted.map((item) => {
    let dependsOn = new Set<string>();
    try {
      const parsed = JSON.parse(item.depends_on_item_ids || '[]');
      if (Array.isArray(parsed)) {
        dependsOn = new Set(parsed.map(String));
      }
    } catch {
      dependsOn = new Set();
    }
    return {
      key: item.id,
      title: item.title,
      description: item.description || '',
      dependsOn,
    };
  });
}

function toPayload(items: LocalItem[]): ProposalItemInput[] {
  const indexByKey = new Map(items.map((item, index) => [item.key, index]));
  return items.map((item, index) => ({
    title: item.title,
    description: item.description || null,
    sort_order: BigInt(index),
    depends_on_indices: [...item.dependsOn]
      .map((depKey) => indexByKey.get(depKey))
      .filter((depIndex): depIndex is number => depIndex !== undefined)
      .map((depIndex) => BigInt(depIndex)),
  }));
}

/** Poll cadence while a breakdown run is in flight. */
const RUNNING_POLL_MS = 3000;

/**
 * Poll interval for the proposal query: `RUNNING_POLL_MS` while a run is in
 * flight, `false` otherwise.
 *
 * A breakdown run completes server-side with nothing pushed to the client, and
 * the app's query defaults (`staleTime` 5min, `refetchOnWindowFocus: false`)
 * mean an un-polled query never notices — the running spinner would sit there
 * until the cache went stale, and closing and reopening the dialog would not
 * help. Exported for direct testing.
 */
export function runningPollInterval(
  data: BreakdownWithItems | null | undefined
): number | false {
  return data?.proposal?.status === 'draft' &&
    data.items.length === 0 &&
    !!data.proposal.execution_process_id
    ? RUNNING_POLL_MS
    : false;
}

const BreakdownReviewDialogImpl = NiceModal.create<BreakdownReviewDialogProps>(
  ({ taskId, projectId }) => {
    const modal = useModal();
    const { t } = useTranslation('tasks');
    const {
      proposal,
      items: remoteItems,
      isLoading: isProposalLoading,
      error: proposalError,
      refetch: refetchProposal,
    } = useBreakdownProposal(taskId, {
      enabled: modal.visible,
      // Poll ONLY while a run is in flight. The run finishes server-side with
      // nothing pushed to the client, and the app's query defaults (staleTime
      // 5min, refetchOnWindowFocus off) mean an un-polled query never notices —
      // the spinner would sit there until the cache went stale, and closing and
      // reopening the dialog would not help. Stops as soon as items arrive or
      // the proposal leaves the running shape, so an idle dialog does not poll.
      refetchInterval: runningPollInterval,
    });
    const { putItems, discard, retry, accept } = useBreakdownMutations(
      taskId,
      projectId,
      {
        onAcceptSuccess: () => modal.remove(),
        onDiscardSuccess: () => modal.remove(),
      }
    );

    const [items, setItems] = useState<LocalItem[]>(() =>
      toLocalItems(remoteItems)
    );

    // Resync local editable state whenever the server-side items change
    // (initial load, or after a putItems/retry round-trip).
    useEffect(() => {
      setItems(toLocalItems(remoteItems));
    }, [remoteItems]);

    const isSaving = putItems.isPending;
    const isAccepting = accept.isPending;

    const commit = (next: LocalItem[]) => {
      setItems(next);
      if (!proposal) return;
      putItems.mutate({
        proposalId: proposal.id,
        payload: { items: toPayload(next) },
      });
    };

    const handleTitleBlur = (index: number, value: string) => {
      if (items[index]?.title === value) return;
      const next = items.map((item, i) =>
        i === index ? { ...item, title: value } : item
      );
      commit(next);
    };

    const handleDescriptionBlur = (index: number, value: string) => {
      if (items[index]?.description === value) return;
      const next = items.map((item, i) =>
        i === index ? { ...item, description: value } : item
      );
      commit(next);
    };

    const handleDelete = (index: number) => {
      const removedKey = items[index]?.key;
      const next = items
        .filter((_, i) => i !== index)
        .map((item) => {
          if (!removedKey || !item.dependsOn.has(removedKey)) return item;
          const dependsOn = new Set(item.dependsOn);
          dependsOn.delete(removedKey);
          return { ...item, dependsOn };
        });
      commit(next);
    };

    const handleMove = (index: number, direction: -1 | 1) => {
      const targetIndex = index + direction;
      if (targetIndex < 0 || targetIndex >= items.length) return;
      const next = [...items];
      [next[index], next[targetIndex]] = [next[targetIndex], next[index]];
      commit(next);
    };

    const handleDependencyToggle = (index: number, depKey: string) => {
      const next = items.map((item, i) => {
        if (i !== index) return item;
        const dependsOn = new Set(item.dependsOn);
        if (dependsOn.has(depKey)) {
          dependsOn.delete(depKey);
        } else {
          dependsOn.add(depKey);
        }
        return { ...item, dependsOn };
      });
      commit(next);
    };

    const handleAccept = () => {
      if (!proposal) return;
      accept.mutate(proposal.id);
    };

    const handleDiscard = () => {
      if (!proposal) return;
      discard.mutate(proposal.id);
    };

    const handleRetry = () => {
      if (!proposal) return;
      retry.mutate(proposal.id);
    };

    const itemsByKey = useMemo(
      () => new Map(items.map((item) => [item.key, item])),
      [items]
    );

    // Until the query settles, `proposal` is null and `remoteItems` is empty —
    // indistinguishable from "no proposal" unless the query state is consulted.
    const isQuerySettled = !isProposalLoading && !proposalError;
    const isFailed = isQuerySettled && proposal?.status === 'failed';
    const isRunning =
      isQuerySettled &&
      !isFailed &&
      proposal?.status === 'draft' &&
      items.length === 0 &&
      !!proposal?.execution_process_id;

    const acceptDisabled =
      !isQuerySettled || items.length === 0 || isSaving || isAccepting;

    return (
      <Dialog
        open={modal.visible}
        onOpenChange={(open) => {
          if (!open) modal.remove();
        }}
      >
        <DialogContent className="sm:max-w-[640px]">
          <DialogHeader>
            <DialogTitle>
              {t('breakdown.title', 'Review task breakdown')}
            </DialogTitle>
          </DialogHeader>

          {isProposalLoading && (
            <div
              className="flex items-center gap-2 py-8 justify-center text-muted-foreground"
              data-testid="breakdown-loading-state"
            >
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>{t('breakdown.loading', 'Loading breakdown...')}</span>
            </div>
          )}

          {!isProposalLoading && proposalError && (
            <Alert variant="destructive" data-testid="breakdown-load-error">
              <div className="space-y-2">
                <p>
                  {t(
                    'breakdown.loadFailed',
                    'Could not load the breakdown proposal.'
                  )}
                </p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => refetchProposal()}
                >
                  {t('breakdown.reload', 'Reload')}
                </Button>
              </div>
            </Alert>
          )}

          {isFailed && (
            <Alert variant="destructive" data-testid="breakdown-failed-banner">
              <div className="space-y-2">
                <p>
                  {proposal?.error ||
                    t('breakdown.failedGeneric', 'Breakdown failed.')}
                </p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={handleRetry}
                  disabled={retry.isPending}
                >
                  {t('breakdown.retry', 'Retry')}
                </Button>
              </div>
            </Alert>
          )}

          {isRunning && (
            <div
              className="flex items-center gap-2 py-8 justify-center text-muted-foreground"
              data-testid="breakdown-running-state"
            >
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>{t('breakdown.running', 'Generating breakdown...')}</span>
            </div>
          )}

          {isQuerySettled && !isFailed && !isRunning && (
            <div className="space-y-4 max-h-[60vh] overflow-y-auto">
              {items.map((item, index) => (
                <div
                  key={item.key}
                  data-testid={`breakdown-item-${item.key}`}
                  className="space-y-2 rounded-md border p-3"
                >
                  <div className="flex items-center gap-2">
                    <Input
                      defaultValue={item.title}
                      onBlur={(e) => handleTitleBlur(index, e.target.value)}
                      aria-label={t('breakdown.itemTitle', 'Item title')}
                      disabled={isSaving}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={t('breakdown.moveUp', 'Move up')}
                      onClick={() => handleMove(index, -1)}
                      disabled={index === 0 || isSaving}
                    >
                      <ArrowUp className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={t('breakdown.moveDown', 'Move down')}
                      onClick={() => handleMove(index, 1)}
                      disabled={index === items.length - 1 || isSaving}
                    >
                      <ArrowDown className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-destructive"
                      aria-label={t('breakdown.deleteItem', 'Delete item')}
                      onClick={() => handleDelete(index)}
                      disabled={isSaving}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>

                  <Textarea
                    defaultValue={item.description}
                    onBlur={(e) => handleDescriptionBlur(index, e.target.value)}
                    aria-label={t(
                      'breakdown.itemDescription',
                      'Item description'
                    )}
                    disabled={isSaving}
                    rows={2}
                  />

                  {item.dependsOn.size > 0 && (
                    <div
                      className="flex flex-wrap gap-1"
                      data-testid={`breakdown-item-${item.key}-deps`}
                    >
                      {[...item.dependsOn].map((depKey) => {
                        const depItem = itemsByKey.get(depKey);
                        if (!depItem) return null;
                        return (
                          <span
                            key={depKey}
                            className="text-xs rounded-full bg-muted px-2 py-0.5"
                          >
                            {depItem.title}
                          </span>
                        );
                      })}
                    </div>
                  )}

                  {items.length > 1 && (
                    <div className="space-y-1">
                      <p className="text-xs uppercase text-muted-foreground">
                        {t('breakdown.dependencies', 'Dependencies')}
                      </p>
                      <div className="flex flex-col gap-1">
                        {items
                          .filter((other) => other.key !== item.key)
                          .map((other) => (
                            <label
                              key={other.key}
                              className="flex items-center gap-2 text-sm"
                            >
                              <Checkbox
                                checked={item.dependsOn.has(other.key)}
                                onCheckedChange={() =>
                                  handleDependencyToggle(index, other.key)
                                }
                                disabled={isSaving}
                              />
                              {other.title}
                            </label>
                          ))}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          <DialogFooter>
            <Button
              variant="destructive"
              onClick={handleDiscard}
              disabled={discard.isPending}
            >
              {t('breakdown.discard', 'Discard')}
            </Button>
            <Button onClick={handleAccept} disabled={acceptDisabled}>
              {t('breakdown.accept', 'Accept')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const BreakdownReviewDialog = defineModal<
  BreakdownReviewDialogProps,
  void
>(BreakdownReviewDialogImpl);

export default BreakdownReviewDialog;
