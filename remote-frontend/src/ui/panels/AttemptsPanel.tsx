import { Badge } from '@/components/core';

export type AttemptRow = { id: string; agent: string; state: 'running' | 'failed' | 'merged'; when: string };

export interface AttemptsPanelProps {
  attempts: AttemptRow[];
}

/** Ported from design-source panels.jsx:131-151 (AttemptsPanel). SEED data removed — caller supplies `attempts`. */
export function AttemptsPanel({ attempts }: AttemptsPanelProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {attempts.map((a) => (
        <div
          key={a.id}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            padding: 12,
            border: '1px solid var(--border)',
            borderRadius: 'var(--radius-md)',
          }}
        >
          {a.state === 'running' ? (
            <span className="vks-loader" style={{ width: 14, height: 14 }} />
          ) : (
            <span
              style={{
                width: 9,
                height: 9,
                borderRadius: '50%',
                background: a.state === 'failed' ? 'var(--danger)' : 'var(--console-success)',
              }}
            />
          )}
          <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', flex: 1 }}>{a.agent}</span>
          <Badge variant="outline">{a.state}</Badge>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--text-dim)' }}>{a.when}</span>
        </div>
      ))}
    </div>
  );
}
