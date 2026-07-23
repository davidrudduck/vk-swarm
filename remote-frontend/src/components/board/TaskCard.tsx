import type { HTMLAttributes, ReactElement } from 'react';
import { cn } from '@/lib/utils';
import { Badge, Loader } from '@/components/core';
import type { TaskStatus } from './StatusBadge';

export type { TaskStatus };
export type AttemptState = 'running' | 'merged' | 'failed';

export interface TaskCardProps extends HTMLAttributes<HTMLDivElement> {
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

function AttemptIndicator({ state }: { state: AttemptState }): ReactElement {
  if (state === 'running') {
    return <Loader size={14} aria-label="Running" />;
  }
  const color = state === 'merged' ? 'var(--success)' : 'var(--danger)';
  const path =
    state === 'merged' ? (
      <path
        d="M5 8.5l2 2 4-4.5"
        stroke={color}
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
      />
    ) : (
      <>
        <path d="M6 6l4 4M10 6l-4 4" stroke={color} strokeWidth="1.6" strokeLinecap="round" />
      </>
    );
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" style={{ flexShrink: 0 }} aria-label={state}>
      <circle cx="8" cy="8" r="7" stroke={color} strokeWidth="1.3" fill="none" opacity={0.5} />
      {path}
    </svg>
  );
}

/**
 * Kanban task card with left status strip. Composes title, description, node
 * tag, labels and an attempt indicator.
 */
export function TaskCard({
  title,
  description,
  status = 'todo',
  node,
  labels = [],
  attempt,
  days,
  className,
  ...props
}: TaskCardProps): ReactElement {
  return (
    <div className={cn('vks-task', `vks-task--${status}`, className)} {...props}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 8 }}>
        <p className="vks-task__title">{title}</p>
        {attempt && <AttemptIndicator state={attempt} />}
      </div>
      {description && (
        <p className="vks-task__desc" title={description}>
          {description}
        </p>
      )}
      <div className="vks-task__meta">
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
          {node && <span className="vks-task__node">{node}</span>}
          {labels.slice(0, 2).map((l) => (
            <Badge key={l} variant="outline" style={{ padding: '1px 7px', fontSize: 'var(--text-xs)' }}>
              {l}
            </Badge>
          ))}
        </div>
        {days != null && (
          <Badge
            variant="secondary"
            style={{ padding: '1px 7px', fontSize: 'var(--text-xs)' }}
            title="Days in column"
          >
            {days}d
          </Badge>
        )}
      </div>
    </div>
  );
}
