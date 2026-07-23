import type { ReactElement } from 'react';
import { TaskCard, type AttemptState, type TaskStatus } from '@/components/board';
import { COLUMNS, type ColumnDef } from './columns';

export interface TaskRow {
  id: string;
  title: string;
  description?: string;
  node?: string;
  labels?: string[];
  attempt?: AttemptState;
  days?: number;
}

export interface BoardViewProps {
  columns: Record<TaskStatus, TaskRow[]>;
  onAdd: (status: TaskStatus) => void;
  onOpen: (task: TaskRow, status: TaskStatus) => void;
  selectedId?: string;
}

interface ColumnHeaderProps {
  col: ColumnDef;
  count: number;
  onAdd: () => void;
}

function ColumnHeader({ col, count, onAdd }: ColumnHeaderProps): ReactElement {
  return (
    <div
      style={{
        position: 'sticky',
        top: 0,
        zIndex: 2,
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '10px 12px',
        background: 'var(--background)',
        borderBottom: '1px dashed var(--border)',
        backgroundImage: `linear-gradient(color-mix(in srgb, ${col.color} 8%, transparent), transparent)`,
      }}
    >
      <span
        style={{
          width: 9,
          height: 9,
          borderRadius: '50%',
          background: col.color,
          flexShrink: 0,
        }}
      />
      <span style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{col.label}</span>
      <span
        style={{
          fontSize: 'var(--text-xs)',
          color: 'var(--text-muted)',
          background: 'var(--surface-card)',
          padding: '1px 7px',
          borderRadius: 4,
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {count}
      </span>
      <div style={{ flex: 1 }} />
      <button
        className="vks-btn vks-btn--ghost"
        style={{ height: 24, width: 24, padding: 0 }}
        onClick={onAdd}
        title="Add task"
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M8 3v10M3 8h10"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
          />
        </svg>
      </button>
    </div>
  );
}

export function BoardView({ columns, onAdd, onOpen, selectedId }: BoardViewProps): ReactElement {
  return (
    <div
      style={{
        display: 'grid',
        gridAutoFlow: 'column',
        gridAutoColumns: 'minmax(264px, 1fr)',
        height: '100%',
        overflowX: 'auto',
        borderLeft: '1px solid var(--border)',
      }}
    >
      {COLUMNS.map((col) => (
        <div
          key={col.key}
          style={{
            display: 'flex',
            flexDirection: 'column',
            borderRight: '1px solid var(--border)',
            minHeight: 0,
          }}
        >
          <ColumnHeader col={col} count={columns[col.key].length} onAdd={() => onAdd(col.key)} />
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 8,
              padding: 10,
              overflowY: 'auto',
              flex: 1,
            }}
          >
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
                style={
                  selectedId === t.id
                    ? { boxShadow: '0 0 0 2px var(--primary)', borderColor: 'var(--primary)' }
                    : undefined
                }
              />
            ))}
            {columns[col.key].length === 0 && (
              <div
                className="vks-ansi-dither vks-scanlines"
                style={{
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--border)',
                  minHeight: 80,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'var(--text-muted)',
                  fontSize: 'var(--text-xs)',
                  fontFamily: 'var(--font-code)',
                  letterSpacing: '0.06em',
                }}
              >
                ░▒ no tasks ▒░
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
