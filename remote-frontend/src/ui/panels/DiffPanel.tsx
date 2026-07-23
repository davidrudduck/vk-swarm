export type DiffLine = { t: 'meta' | 'ctx' | 'add' | 'del'; s: string };

export interface DiffPanelProps {
  lines: DiffLine[];
}

const COLOR: Record<DiffLine['t'], string> = {
  meta: 'var(--text-muted)',
  ctx: 'var(--text-muted)',
  add: 'var(--console-success)',
  del: 'var(--console-error)',
};

const BG: Partial<Record<DiffLine['t'], string>> = {
  add: 'hsl(var(--vks-emerald-hsl) / 0.08)',
  del: 'hsl(var(--vks-coral-hsl) / 0.08)',
};

/** Ported from design-source panels.jsx:93-112 (DiffPanel). SEED data removed — caller supplies `lines`. */
export function DiffPanel({ lines }: DiffPanelProps) {
  return (
    <div
      style={{
        background: 'var(--console-bg)',
        border: '1px solid var(--border)',
        borderRadius: 'var(--radius-md)',
        overflow: 'hidden',
        fontFamily: 'var(--font-code)',
        fontSize: 'var(--text-sm)',
      }}
    >
      {lines.map((l, i) => (
        <div
          key={i}
          style={{
            padding: '3px 12px',
            color: COLOR[l.t],
            background: BG[l.t] ?? 'transparent',
            whiteSpace: 'pre',
            fontWeight: l.t === 'meta' ? 600 : 400,
          }}
        >
          {l.s}
        </div>
      ))}
    </div>
  );
}
