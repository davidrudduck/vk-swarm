// VK-Swarm UI kit — Nodes view + Task detail drawer + Processes placeholder.
const { useState } = window.React;

function NodesView() {
  const { NodeCard, Badge, Button } = window.VKSwarmDesignSystem_067861;
  return (
    <div style={{ padding: 20, overflowY: 'auto', height: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <h2 style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)', fontWeight: 600, margin: 0 }}>Hive</h2>
        <span className="vks-status vks-status--done"><span className="vks-status__dot" /><span style={{ fontSize: 'var(--text-sm)' }}>3 nodes online</span></span>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: 12, maxWidth: 1000 }}>
        <NodeCard name="justX.raverx.net" os="mac" online meta="3 agents · wss://hive.raverx.net" right={<Badge variant="secondary" dot>3</Badge>} />
        <NodeCard name="linux-01" os="linux" online meta="1 agent · streaming logs" right={<Badge variant="secondary" dot>1</Badge>} />
        <NodeCard name="winbox" os="windows" online meta="2 agents · direct connect" right={<Badge variant="secondary" dot>2</Badge>} />
        <NodeCard name="ci-runner-04" os="linux" online={false} meta="last seen 4m ago" right={<Badge variant="outline">offline</Badge>} />
      </div>
    </div>
  );
}

function ProcessesView() {
  const rows = [
    { name: 'claude-code · feat/auth', node: 'justX', state: 'running', dur: '2m 14s' },
    { name: 'dev-server · vite', node: 'justX', state: 'running', dur: '41m' },
    { name: 'codex · diff-virtualization', node: 'winbox', state: 'running', dur: '58s' },
    { name: 'pnpm test', node: 'linux-01', state: 'done', dur: '1m 02s' },
  ];
  return (
    <div style={{ padding: 20, overflowY: 'auto', height: '100%' }}>
      <h2 style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)', fontWeight: 600, margin: '0 0 14px' }}>Processes</h2>
      <div className="vks-card" style={{ overflow: 'hidden', maxWidth: 860 }}>
        {rows.map((r, i) => (
          <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 16px', borderBottom: i < rows.length - 1 ? '1px solid var(--border)' : 0 }}>
            {r.state === 'running'
              ? <span className="vks-loader" style={{ width: 14, height: 14 }} />
              : <span className="vks-status vks-status--done"><span className="vks-status__dot" /></span>}
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', flex: 1 }}>{r.name}</span>
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)' }}>{r.node}</span>
            <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-dim)', width: 56, textAlign: 'right' }}>{r.dur}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function TaskDrawer({ task, status, onClose }) {
  const { Button, Badge, StatusBadge, Tabs } = window.VKSwarmDesignSystem_067861;
  const [tab, setTab] = useState('diff');
  if (!task) return null;
  return (
    <>
      <div onClick={onClose} style={{ position: 'absolute', inset: 0, background: 'var(--surface-overlay)', zIndex: 10 }} />
      <aside style={{
        position: 'absolute', top: 0, right: 0, bottom: 0, width: 460, maxWidth: '90%', zIndex: 11,
        background: 'var(--surface-card)', borderLeft: '1px solid var(--border-strong)', boxShadow: 'var(--shadow-lg)',
        display: 'flex', flexDirection: 'column',
      }}>
        <div style={{ padding: '16px 18px', borderBottom: '1px solid var(--border)' }}>
          <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10 }}>
            <StatusBadge status={status} showLabel={false} />
            <h3 style={{ fontSize: 'var(--text-lg)', fontWeight: 600, margin: 0, flex: 1, lineHeight: 1.3 }}>{task.title}</h3>
            <button className="vks-btn vks-btn--ghost vks-btn--icon" onClick={onClose} style={{ height: 28, width: 28 }}>
              <window.Icon d={<><path d="M6 6l12 12M18 6L6 18" /></>} size={16} />
            </button>
          </div>
          <div style={{ display: 'flex', gap: 6, marginTop: 12, flexWrap: 'wrap' }}>
            <Badge variant="outline" dot>{status === 'inprogress' ? 'In Progress' : status}</Badge>
            <Badge variant="secondary">{task.node}</Badge>
            {(task.labels || []).map((l) => <Badge key={l} variant="outline">{l}</Badge>)}
          </div>
        </div>
        <div style={{ padding: '14px 18px' }}>
          <Tabs value={tab} onValueChange={setTab} tabs={[{ value: 'diff', label: 'Diff' }, { value: 'logs', label: 'Logs' }, { value: 'attempts', label: 'Attempts' }]} />
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '0 18px 18px' }}>
          {tab === 'diff' && <DiffPanel />}
          {tab === 'logs' && <LogsPanel node={task.node} />}
          {tab === 'attempts' && <AttemptsPanel />}
        </div>
        <div style={{ padding: 16, borderTop: '1px solid var(--border)', display: 'flex', gap: 8 }}>
          <Button variant="primary" size="sm" style={{ flex: 1 }}>Merge</Button>
          <Button variant="outline" size="sm">Rebase</Button>
          <Button variant="ghost" size="sm">Open in IDE</Button>
        </div>
      </aside>
    </>
  );
}

function DiffPanel() {
  const lines = [
    { t: 'meta', s: 'src/auth/callback.ts' },
    { t: 'ctx', s: '  export async function handleCallback(req) {' },
    { t: 'del', s: "-   const token = req.query.code;" },
    { t: 'add', s: "+   const token = await exchangeCode(req.query.code);" },
    { t: 'add', s: "+   await persistSession(token);" },
    { t: 'ctx', s: '    return redirect("/projects");' },
    { t: 'ctx', s: '  }' },
  ];
  const color = { meta: 'var(--text-muted)', ctx: 'var(--text-muted)', add: 'var(--console-success)', del: 'var(--console-error)' };
  const bg = { add: 'hsl(var(--vks-emerald-hsl) / 0.08)', del: 'hsl(var(--vks-coral-hsl) / 0.08)' };
  return (
    <div style={{ background: 'var(--console-bg)', border: '1px solid var(--border)', borderRadius: 'var(--radius-md)', overflow: 'hidden', fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)' }}>
      {lines.map((l, i) => (
        <div key={i} style={{ padding: '3px 12px', color: color[l.t], background: bg[l.t] || 'transparent', whiteSpace: 'pre', fontWeight: l.t === 'meta' ? 600 : 400 }}>{l.s}</div>
      ))}
    </div>
  );
}

function LogsPanel({ node }) {
  const lines = [
    ['muted', `→ connecting to node ${node}`],
    ['ok', '✓ worktree created · branch feat/auth'],
    ['fg', '$ claude-code run'],
    ['muted', '  reading 14 files…'],
    ['cy', '  editing src/auth/callback.ts'],
    ['ok', '✓ applied 2 edits'],
    ['err', '✗ test failed: expected session to persist'],
  ];
  const map = { muted: 'var(--text-muted)', ok: 'var(--console-success)', err: 'var(--console-error)', cy: 'var(--vks-cyan)', fg: 'var(--foreground)' };
  return (
    <div style={{ background: 'var(--console-bg)', border: '1px solid var(--border)', borderRadius: 'var(--radius-md)', padding: '12px 14px', fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', lineHeight: 1.7 }}>
      {lines.map((l, i) => <div key={i} style={{ color: map[l[0]] }}>{l[1]}</div>)}
    </div>
  );
}

function AttemptsPanel() {
  const { Badge } = window.VKSwarmDesignSystem_067861;
  const items = [
    { agent: 'claude-code', state: 'running', when: 'now' },
    { agent: 'codex', state: 'failed', when: '8m ago' },
  ];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {items.map((a, i) => (
        <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: 12, border: '1px solid var(--border)', borderRadius: 'var(--radius-md)' }}>
          {a.state === 'running' ? <span className="vks-loader" style={{ width: 14, height: 14 }} /> : <span style={{ width: 9, height: 9, borderRadius: '50%', background: 'var(--danger)' }} />}
          <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', flex: 1 }}>{a.agent}</span>
          <Badge variant="outline">{a.state}</Badge>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--text-dim)' }}>{a.when}</span>
        </div>
      ))}
    </div>
  );
}

Object.assign(window, { NodesView, ProcessesView, TaskDrawer });
