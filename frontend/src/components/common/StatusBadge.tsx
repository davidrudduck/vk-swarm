import type { TaskStatus } from 'shared/types';
import { cn } from '@/lib/utils';

interface StatusBadgeProps {
  status: TaskStatus;
  /** When true, append the status label text after the dot. */
  showLabel?: boolean;
  className?: string;
}

// Static literal class strings so Tailwind's content scan generates the
// arbitrary-value classes (a template-literal `--status-${status}` would not
// be emitted). Tokens (bare HSL triplets) come from task 002 — wrap in hsl().
// TODO(i18n): vk-swarm-node-ui-localize — labels are English literals.
const statusConfig: Record<TaskStatus, { dotClass: string; label: string }> = {
  todo: { dotClass: 'bg-[hsl(var(--status-todo))]', label: 'Todo' },
  inprogress: {
    dotClass: 'bg-[hsl(var(--status-inprogress))]',
    label: 'In Progress',
  },
  inreview: { dotClass: 'bg-[hsl(var(--status-inreview))]', label: 'In Review' },
  done: { dotClass: 'bg-[hsl(var(--status-done))]', label: 'Done' },
  cancelled: {
    dotClass: 'bg-[hsl(var(--status-cancelled))]',
    label: 'Cancelled',
  },
};

/**
 * Status indicator: an 8px coloured dot (driven by the --status-* tokens) with
 * an optional trailing label. Mirrors ConnectionStatusBadge's config-map +
 * cn() conventions.
 */
export function StatusBadge({
  status,
  showLabel = false,
  className,
}: StatusBadgeProps) {
  const config = statusConfig[status];

  return (
    <span
      className={cn('inline-flex items-center gap-1.5', className)}
    >
      <span
        className={cn('h-2 w-2 rounded-full shrink-0', config.dotClass)}
        aria-hidden="true"
      />
      {showLabel && (
        <span className="text-xs font-medium">{config.label}</span>
      )}
    </span>
  );
}
