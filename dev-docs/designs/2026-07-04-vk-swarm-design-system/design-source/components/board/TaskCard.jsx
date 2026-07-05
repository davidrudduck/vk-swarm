import React from 'react';

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
  attempt, // 'running' | 'merged' | 'failed' | undefined
  days,
  className = '',
  ...props
}) {
  const cls = ['vks-task', `vks-task--${status}`, className].filter(Boolean).join(' ');
  return (
    <div className={cls} {...props}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 8 }}>
        <p className="vks-task__title">{title}</p>
        {attempt && <AttemptIndicator attempt={attempt} />}
      </div>
      {description && <p className="vks-task__desc" title={description}>{description}</p>}
      <div className="vks-task__meta">
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
          {node && <span className="vks-task__node">{node}</span>}
          {labels.slice(0, 2).map((l) => (
            <span key={l} className="vks-badge vks-badge--outline" style={{ padding: '1px 7px', fontSize: 'var(--text-xs)' }}>{l}</span>
          ))}
        </div>
        {days != null && (
          <span className="vks-badge vks-badge--secondary" style={{ padding: '1px 7px', fontSize: 'var(--text-xs)' }} title="Days in column">{days}d</span>
        )}
      </div>
    </div>
  );
}

function AttemptIndicator({ attempt }) {
  if (attempt === 'running') return <span className="vks-loader" style={{ width: 14, height: 14, flexShrink: 0 }} aria-label="Running" />;
  const color = attempt === 'merged' ? 'var(--success)' : 'var(--danger)';
  const path = attempt === 'merged'
    ? <path d="M5 8.5l2 2 4-4.5" stroke={color} strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" fill="none" />
    : <><path d="M6 6l4 4M10 6l-4 4" stroke={color} strokeWidth="1.6" strokeLinecap="round" /></>;
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" style={{ flexShrink: 0 }} aria-label={attempt}>
      <circle cx="8" cy="8" r="7" stroke={color} strokeWidth="1.3" fill="none" opacity="0.5" />
      {path}
    </svg>
  );
}
