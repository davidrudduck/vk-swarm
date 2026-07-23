import type { ReactElement } from 'react';

export interface ProcessRow {
  id: string;
  name: string;
  node: string;
  state: 'running' | 'done';
  dur: string;
}

export interface ProcessesViewProps {
  processes: ProcessRow[];
}

/** Running/recently-finished process list panel. */
export function ProcessesView({ processes }: ProcessesViewProps): ReactElement {
  return (
    <div style={{ padding: 20, overflowY: 'auto', height: '100%' }}>
      <h2 style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)', fontWeight: 600, margin: '0 0 14px' }}>
        Processes
      </h2>
      <div className="vks-card" style={{ overflow: 'hidden', maxWidth: 860 }}>
        {processes.map((r, i) => (
          <div
            key={r.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '12px 16px',
              borderBottom: i < processes.length - 1 ? '1px solid var(--border)' : 0,
            }}
          >
            {r.state === 'running' ? (
              <span className="vks-loader" style={{ width: 14, height: 14 }} />
            ) : (
              <span className="vks-status vks-status--done">
                <span className="vks-status__dot" />
              </span>
            )}
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', flex: 1 }}>{r.name}</span>
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)' }}>
              {r.node}
            </span>
            <span
              style={{
                fontFamily: 'var(--font-code)',
                fontSize: 'var(--text-xs)',
                color: 'var(--text-dim)',
                width: 56,
                textAlign: 'right',
              }}
            >
              {r.dur}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
