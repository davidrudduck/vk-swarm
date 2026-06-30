// VK-Swarm UI kit — Kanban board view. Uses TaskCard from the bundle.
const { useState } = window.React;

const SEED = {
  todo: [
    { id: 't1', title: 'Add rate limiting to hive WebSocket', description: 'Throttle node reconnect storms during deploys', node: 'justX', labels: ['infra'], days: 1 },
    { id: 't2', title: 'Document swarm-hive setup', description: 'Walk through VK_HIVE_URL and node API keys', node: 'linux-01', labels: ['docs'], days: 3 },
  ],
  inprogress: [
    { id: 't3', title: 'Wire up OAuth callback', description: 'Handle redirect and persist the session token', node: 'justX', labels: ['auth', 'backend'], attempt: 'running', days: 2 },
    { id: 't4', title: 'Diff view virtualization', description: 'Render large diffs without jank', node: 'winbox', labels: ['ui'], attempt: 'running', days: 1 },
  ],
  inreview: [
    { id: 't5', title: 'Migrate hive schema to pgvector', description: 'Embedding columns + backfill job', node: 'linux-01', labels: ['db'], attempt: 'failed', days: 4 },
  ],
  done: [
    { id: 't6', title: 'Add DiffViewSwitch component', node: 'justX', labels: ['ui'], attempt: 'merged', days: 6 },
    { id: 't7', title: 'Compact label list on cards', node: 'winbox', labels: ['ui'], attempt: 'merged', days: 8 },
  ],
  cancelled: [
    { id: 't8', title: 'Experiment: local SQLite-only mode', node: 'justX', days: 12 },
  ],
};

const COLUMNS = [
  { key: 'todo', label: 'To Do', color: 'var(--status-todo)' },
  { key: 'inprogress', label: 'In Progress', color: 'var(--status-inprogress)' },
  { key: 'inreview', label: 'In Review', color: 'var(--status-inreview)' },
  { key: 'done', label: 'Done', color: 'var(--status-done)' },
  { key: 'cancelled', label: 'Cancelled', color: 'var(--status-cancelled)' },
];

function ColumnHeader({ col, count, onAdd }) {
  return (
    <div style={{
      position: 'sticky', top: 0, zIndex: 2, display: 'flex', alignItems: 'center', gap: 8,
      padding: '10px 12px', background: 'var(--background)', borderBottom: '1px dashed var(--border)',
      backgroundImage: `linear-gradient(color-mix(in srgb, ${col.color} 8%, transparent), transparent)`,
    }}>
      <span style={{ width: 9, height: 9, borderRadius: '50%', background: col.color, flexShrink: 0 }} />
      <span style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{col.label}</span>
      <span style={{ fontSize: 'var(--text-xs)', color: 'var(--text-muted)', background: 'var(--surface-card)', padding: '1px 7px', borderRadius: 4, fontVariantNumeric: 'tabular-nums' }}>{count}</span>
      <div style={{ flex: 1 }} />
      <button className="vks-btn vks-btn--ghost" style={{ height: 24, width: 24, padding: 0 }} onClick={onAdd} title="Add task">
        <window.Icon d={window.ICONS.plus} size={14} />
      </button>
    </div>
  );
}

function BoardView({ columns, onAdd, onOpen, selectedId }) {
  const { TaskCard } = window.VKSwarmDesignSystem_067861;
  return (
    <div style={{ display: 'grid', gridAutoFlow: 'column', gridAutoColumns: 'minmax(264px, 1fr)', height: '100%', overflowX: 'auto', borderLeft: '1px solid var(--border)' }}>
      {COLUMNS.map((col) => (
        <div key={col.key} style={{ display: 'flex', flexDirection: 'column', borderRight: '1px solid var(--border)', minHeight: 0 }}>
          <ColumnHeader col={col} count={columns[col.key].length} onAdd={() => onAdd(col.key)} />
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: 10, overflowY: 'auto', flex: 1 }}>
            {columns[col.key].map((t) => (
              <TaskCard
                key={t.id}
                title={t.title}
                description={t.description}
                status={col.key}
                node={t.node}
                labels={t.labels}
                attempt={t.attempt}
                days={t.days}
                onClick={() => onOpen(t, col.key)}
                style={selectedId === t.id ? { boxShadow: '0 0 0 2px var(--primary)', borderColor: 'var(--primary)' } : null}
              />
            ))}
            {columns[col.key].length === 0 && (
              <div className="vks-ansi-dither vks-scanlines" style={{ borderRadius: 'var(--radius-md)', border: '1px solid var(--border)', minHeight: 80, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', fontSize: 'var(--text-xs)', fontFamily: 'var(--font-code)', letterSpacing: '0.06em' }}>
                ░▒ no tasks ▒░
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

Object.assign(window, { BoardView, SEED, COLUMNS });
