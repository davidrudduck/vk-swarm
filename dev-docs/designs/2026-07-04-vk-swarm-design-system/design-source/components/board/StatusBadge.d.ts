import * as React from 'react';

export type TaskStatus = 'todo' | 'inprogress' | 'inreview' | 'done' | 'cancelled';

export interface StatusBadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Kanban task status. @default 'todo' */
  status?: TaskStatus;
  /** Show the text label beside the dot. @default true */
  showLabel?: boolean;
  /** Override the default label text. */
  label?: React.ReactNode;
}

/** Colored dot + label for the five VK-Swarm task statuses. */
export function StatusBadge(props: StatusBadgeProps): React.ReactElement;
