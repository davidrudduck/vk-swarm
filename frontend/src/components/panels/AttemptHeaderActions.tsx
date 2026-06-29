import { X } from 'lucide-react';
import { Button } from '../ui/button';
import { Tabs, TabsList, TabsTrigger } from '../ui/tabs';
import type { LayoutMode } from '../layout/TasksLayout';
import type { TaskAttempt, TaskWithAttemptStatus, Label } from 'shared/types';
import { ActionsDropdown } from '../ui/actions-dropdown';
import { useIsOrgAdmin, useRemoteConnectionStatus } from '@/hooks';
import { useTaskLabels } from '@/hooks/useTaskLabels';
import { ConnectionStatusBadge } from '@/components/common/ConnectionStatusBadge';
import { StatusBadge } from '@/components/common/StatusBadge';
import { Badge } from '@/components/ui/badge';
import { LabelBadge } from '@/components/labels/LabelBadge';

interface AttemptHeaderActionsProps {
  onClose: () => void;
  mode?: LayoutMode;
  onModeChange?: (mode: LayoutMode) => void;
  task: TaskWithAttemptStatus;
  attempt?: TaskAttempt | null;
  isMobile?: boolean;
}

export const AttemptHeaderActions = ({
  onClose,
  mode,
  onModeChange,
  task,
  attempt,
  isMobile,
}: AttemptHeaderActionsProps) => {
  const isOrgAdmin = useIsOrgAdmin();
  // Only fetch connection status for tasks with an in-progress attempt
  // Tasks without active assignments return 404 from Hive, showing confusing "Disconnected" status
  const { status: connectionStatus } = useRemoteConnectionStatus(task, {
    enabled: Boolean(attempt) && task?.has_in_progress_attempt === true,
  });
  // Labels for the SC18 badges row (same hook TaskCard uses — keyed by task.id).
  const { data: labels } = useTaskLabels(task.id, true);

  // Only show connection badge for remote tasks with running attempts
  const showConnectionBadge =
    Boolean(task?.shared_task_id) &&
    task?.has_in_progress_attempt === true &&
    connectionStatus !== 'local';

  return (
    <>
      {/* SC18 chrome: header status dot + badges cluster (status outline+dot, node,
          labels). NewCardHeader renders the actions slot as a top-right inline row
          (new-card.tsx), so this is an inline cluster, not a literal below-header
          band, and the header dot sits here too — see ledger. */}
      <div className="flex items-center gap-1.5">
        {/* SC18:96 — header status dot (no label) */}
        <StatusBadge status={task.status} />
        {/* SC18:97 — row status badge: outline + dot + label */}
        <StatusBadge
          status={task.status}
          showLabel
          className="rounded-full border border-border px-2 py-0.5"
        />
        {task.source_node_name && (
          <Badge variant="secondary">{task.source_node_name}</Badge>
        )}
        {labels?.map((label: Label) => (
          <LabelBadge
            key={label.id}
            label={label}
            variant="outline"
            size="sm"
          />
        ))}
      </div>
      {/* Connection status badge for remote tasks */}
      {showConnectionBadge && (
        <>
          <ConnectionStatusBadge status={connectionStatus} />
          <div className="h-4 w-px bg-border" />
        </>
      )}
      {!isMobile && typeof mode !== 'undefined' && onModeChange && (
        // TODO(i18n): vk-swarm-node-ui-localize — Diff / Logs / Attempts are
        // literal English (the existing attemptHeaderActions.* keys render
        // "Diffs"/"Terminal", not the SC18 labels).
        <Tabs
          value={
            mode === 'diffs'
              ? 'diff'
              : mode === 'terminal'
                ? 'logs'
                : mode === null
                  ? 'attempts'
                  : '_none'
          }
          onValueChange={(v) => {
            onModeChange(v === 'diff' ? 'diffs' : v === 'logs' ? 'terminal' : null);
          }}
          aria-label="Layout mode"
        >
          <TabsList>
            <TabsTrigger value="diff">Diff</TabsTrigger>
            <TabsTrigger value="logs">Logs</TabsTrigger>
            <TabsTrigger value="attempts">Attempts</TabsTrigger>
          </TabsList>
        </Tabs>
      )}
      <ActionsDropdown task={task} attempt={attempt} isOrgAdmin={isOrgAdmin} />
      <Button variant="icon" aria-label="Close" onClick={onClose}>
        <X size={16} />
      </Button>
    </>
  );
};
