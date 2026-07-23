export type LogLine = [kind: 'muted' | 'ok' | 'err' | 'cy' | 'fg', text: string];

export interface LogsPanelProps {
  lines: LogLine[];
}

const MAP: Record<LogLine[0], string> = {
  muted: 'var(--text-muted)',
  ok: 'var(--console-success)',
  err: 'var(--console-error)',
  cy: 'var(--vks-cyan)',
  fg: 'var(--foreground)',
};

/** Ported from design-source panels.jsx:114-129 (LogsPanel). SEED data removed — caller supplies `lines`. */
export function LogsPanel({ lines }: LogsPanelProps) {
  return (
    <div
      style={{
        background: 'var(--console-bg)',
        border: '1px solid var(--border)',
        borderRadius: 'var(--radius-md)',
        padding: '12px 14px',
        fontFamily: 'var(--font-code)',
        fontSize: 'var(--text-sm)',
        lineHeight: 1.7,
      }}
    >
      {lines.map((l, i) => (
        <div key={i} style={{ color: MAP[l[0]] }}>
          {l[1]}
        </div>
      ))}
    </div>
  );
}
