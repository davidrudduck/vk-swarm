import * as React from 'react';

export type TaskStatus = 'todo' | 'inprogress' | 'inreview' | 'done' | 'cancelled';
export type AttemptState = 'running' | 'merged' | 'failed';

export interface TaskCardProps extends React.HTMLAttributes<HTMLDivElement> {
  title: string;
  /** One-line preview; truncated with ellipsis. */
  description?: string;
  /** Drives the left status strip. @default 'todo' */
  status?: TaskStatus;
  /** Short source-node name (e.g. "justX"). */
  node?: string;
  /** Label chips (first two shown). */
  labels?: string[];
  /** Latest attempt state → spinner / check / cross. */
  attempt?: AttemptState;
  /** Days in current column → trailing badge. */
  days?: number;
}

/**
 * Draggable kanban card with a colored left strip per status.
 *
 * @startingPoint section="Board" subtitle="Kanban task card with status strip" viewport="320x140"
 */
export function TaskCard(props: TaskCardProps): React.ReactElement;
