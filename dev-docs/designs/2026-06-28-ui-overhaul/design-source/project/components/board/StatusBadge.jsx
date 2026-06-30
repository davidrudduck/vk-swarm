import React from 'react';

const LABELS = { todo: 'To Do', inprogress: 'In Progress', inreview: 'In Review', done: 'Done', cancelled: 'Cancelled' };

/** Status indicator (dot + label) matching the kanban column colors. */
export function StatusBadge({ status = 'todo', showLabel = true, label, className = '', ...props }) {
  const cls = ['vks-status', `vks-status--${status}`, className].filter(Boolean).join(' ');
  return (
    <span className={cls} {...props}>
      <span className="vks-status__dot" />
      {showLabel && <span>{label ?? LABELS[status] ?? status}</span>}
    </span>
  );
}
