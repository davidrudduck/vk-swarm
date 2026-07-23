import type { HTMLAttributes, ReactElement, ReactNode } from 'react';
import { cn } from '@/lib/utils';

export type TaskStatus = 'todo' | 'inprogress' | 'inreview' | 'done' | 'cancelled';

export interface StatusBadgeProps extends HTMLAttributes<HTMLSpanElement> {
  /** Kanban task status. @default 'todo' */
  status?: TaskStatus;
  /** Show the text label beside the dot. @default true */
  showLabel?: boolean;
  /** Override the default label text. */
  label?: ReactNode;
}

const LABELS: Record<TaskStatus, string> = {
  todo: 'To Do',
  inprogress: 'In Progress',
  inreview: 'In Review',
  done: 'Done',
  cancelled: 'Cancelled',
};

/** Colored dot + label for the five VK-Swarm task statuses. */
export function StatusBadge({
  status = 'todo',
  showLabel = true,
  label,
  className,
  ...props
}: StatusBadgeProps): ReactElement {
  return (
    <span className={cn('vks-status', `vks-status--${status}`, className)} {...props}>
      <span className="vks-status__dot" />
      {showLabel && <span>{label ?? LABELS[status] ?? status}</span>}
    </span>
  );
}
