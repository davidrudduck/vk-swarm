// VK-Swarm UI kit — full-page Settings route.
// Left sidebar (8 sections) + close/ESC, content is a stack of SettingsSection
// cards built from the design-system SettingsRow controls. A single draft
// object drives a sticky dirty save-bar shared across panels.
const { useState, useEffect, useRef } = React;

// ---- Extra lucide-style icons not in chrome.jsx ----------------------------
const SICONS = {
  x: <><path d="M18 6 6 18M6 6l12 12" /></>,
  building: <><path d="M6 22V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v18Z" /><path d="M6 12H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2" /><path d="M10 6h4M10 10h4M10 14h4M10 18h4" /></>,
  network: <><rect x="9" y="2" width="6" height="6" rx="1" /><rect x="2" y="16" width="6" height="6" rx="1" /><rect x="16" y="16" width="6" height="6" rx="1" /><path d="M12 8v4M5 16v-2h14v2" /></>,
  cpu: <><rect x="6" y="6" width="12" height="12" rx="2" /><rect x="9" y="9" width="6" height="6" /><path d="M9 2v2M15 2v2M9 20v2M15 20v2M2 9h2M2 15h2M20 9h2M20 15h2" /></>,
  database: <><ellipse cx="12" cy="5" rx="8" ry="3" /><path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5" /><path d="M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" /></>,
  volume: <><path d="M11 5 6 9H2v6h4l5 4z" /><path d="M15.5 8.5a5 5 0 0 1 0 7" /></>,
  check: <><path d="M20 6 9 17l-5-5" /></>,
  alert: <><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" /><path d="M12 9v4M12 17h.01" /></>,
  trash: <><path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" /></>,
  refresh: <><path d="M3 12a9 9 0 0 1 15-6.7L21 8" /><path d="M21 3v5h-5" /><path d="M21 12a9 9 0 0 1-15 6.7L3 16" /><path d="M3 21v-5h5" /></>,
  key: <><circle cx="7.5" cy="15.5" r="4.5" /><path d="M10.7 12.3 21 2M16 7l3 3M14 9l3 3" /></>,
};

// ---- Nav model -------------------------------------------------------------
function navItems(ICONS) {
  return [
    { id: 'general', icon: ICONS.settings, name: 'General', desc: 'Appearance, editor, notifications' },
    { id: 'projects', icon: ICONS.folder, name: 'Projects', desc: 'Per-project defaults' },
    { id: 'organizations', icon: SICONS.building, name: 'Organizations', desc: 'Members & access' },
    { id: 'swarm', icon: SICONS.network, name: 'Swarm', desc: 'Shared projects, labels, templates' },
    { id: 'agents', icon: SICONS.cpu, name: 'Agents', desc: 'Executor profiles & configs' },
    { id: 'mcp', icon: ICONS.server, name: 'MCP', desc: 'Model Context Protocol servers' },
    { id: 'webhooks', icon: ICONS.bolt, name: 'Webhooks', desc: 'Outbound event delivery' },
    { id: 'system', icon: SICONS.database, name: 'System', desc: 'Backups, hive sync, build info' },
  ];
}

// ---- Default draft ---------------------------------------------------------
const DEFAULT_DRAFT = {
  general: {
    theme: 'DARK', language: 'BROWSER',
    uiFont: 'INTER', codeFont: 'JET_BRAINS_MONO', proseFont: 'SOURCE_SERIF', disableLigatures: false,
    executor: 'CLAUDE_CODE', variant: 'DEFAULT',
    editorType: 'VS_CODE', customCommand: '', remoteHost: '', remoteUser: '',
    terminalFontSize: '14', cursorBlink: true,
    timezone: 'LOCAL', tokenTs: false, tokenTsFormat: '[HH:mm:ss.SSS dd/MM/yyyy]',
    branchPrefix: 'vk', soundEnabled: true, soundFile: 'ROOK', pushEnabled: false,
    initialLoad: '100',
  },
  projects: { defaultBranch: 'main', worktreeBase: '~/.vk-swarm/worktrees', autoCleanup: true, copyEnv: true, setupScript: 'pnpm install' },
  organizations: { org: 'raverx', defaultRole: 'MEMBER', requireReview: true, sso: false },
  swarm: { org: 'raverx', autoSync: true, conflictStrategy: 'MANUAL' },
  agents: { executor: 'CLAUDE_CODE', config: 'DEFAULT', formEditor: true },
  mcp: { scope: 'GLOBAL' },
  webhooks: { enabled: true, retry: '3', secretSet: true },
  system: { autoBackup: true, backupInterval: 'DAILY', retention: '30', telemetry: false },
};

// ---- Small building blocks -------------------------------------------------
function Alert({ variant, icon, children }) {
  return (
    <div className={['vks-alert', variant && 'vks-alert--' + variant].filter(Boolean).join(' ')} role="status">
      {icon && <span className="vks-alert__icon">{icon}</span>}
      <div>{children}</div>
    </div>
  );
}

function sel(opts) { return opts.map((o) => (typeof o === 'string' ? { value: o, label: o } : o)); }

// ---- General panel (the flagship — full fidelity) --------------------------
function GeneralPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Input, Switch, Checkbox, Button } = window.VKSwarmDesignSystem_067861;
  const Icon = window.Icon;
  const g = draft.general;
  const set = (k, v) => patch('general', { [k]: v });

  const prefixError = (() => {
    const p = g.branchPrefix;
    if (!p) return null;
    if (/\s/.test(p)) return 'No spaces allowed.';
    if (p.includes('/')) return 'Cannot contain a slash.';
    if (p.startsWith('.')) return 'Cannot start with a dot.';
    return null;
  })();

  const remoteCapable = ['VS_CODE', 'CURSOR', 'WINDSURF'].includes(g.editorType);

  return (
    <>
      <SettingsSection title="Appearance" description="Customize how VK-Swarm looks on this device.">
        <SettingsRow label="Theme" htmlFor="s-theme" helper="Midnight Terminal is the default. Applied on save.">
          <Select id="s-theme" value={g.theme} onValueChange={(v) => set('theme', v)}
            options={sel([{ value: 'DARK', label: 'Dark' }, { value: 'LIGHT', label: 'Light' }, { value: 'SYSTEM', label: 'System' }])} />
        </SettingsRow>
        <SettingsRow label="Language" htmlFor="s-lang" helper="Interface language for menus and labels.">
          <Select id="s-lang" value={g.language} onValueChange={(v) => set('language', v)}
            options={sel([{ value: 'BROWSER', label: 'Browser Default' }, { value: 'EN', label: 'English' }, { value: 'DE', label: 'Deutsch' }, { value: 'JA', label: '日本語' }])} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Typography" description="Fonts for the interface, code and prose.">
        <SettingsRow label="UI font" htmlFor="s-uifont">
          <Select id="s-uifont" value={g.uiFont} onValueChange={(v) => set('uiFont', v)}
            options={sel([{ value: 'INTER', label: 'Inter' }, { value: 'ROBOTO', label: 'Roboto' }, { value: 'PUBLIC_SANS', label: 'Public Sans' }, { value: 'CHIVO_MONO', label: 'Chivo Mono' }, { value: 'SYSTEM', label: 'System' }])} />
        </SettingsRow>
        <SettingsRow label="Code font" htmlFor="s-codefont" helper="Logs, diffs, branches and node names.">
          <Select id="s-codefont" value={g.codeFont} onValueChange={(v) => set('codeFont', v)}
            options={sel([{ value: 'JET_BRAINS_MONO', label: 'JetBrains Mono' }, { value: 'CASCADIA_MONO', label: 'Cascadia Mono' }, { value: 'HACK', label: 'Hack' }, { value: 'IBM_PLEX_MONO', label: 'IBM Plex Mono' }, { value: 'SYSTEM', label: 'System' }])} />
        </SettingsRow>
        <SettingsRow label="Prose font" htmlFor="s-prosefont">
          <Select id="s-prosefont" value={g.proseFont} onValueChange={(v) => set('proseFont', v)}
            options={sel([{ value: 'SOURCE_SERIF', label: 'Source Serif 4' }, { value: 'INTER', label: 'Inter' }, { value: 'GEORGIA', label: 'Georgia' }, { value: 'SYSTEM', label: 'System' }])} />
        </SettingsRow>
        <SettingsRow inline label="Disable ligatures" htmlFor="s-liga" helper="Render code fonts without programming ligatures.">
          <Checkbox id="s-liga" checked={g.disableLigatures} onCheckedChange={(v) => set('disableLigatures', v)} />
        </SettingsRow>
        <div style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', color: 'var(--text-muted)', background: 'var(--surface-raised)', border: '1px solid var(--border)', borderRadius: 'var(--radius-md)', padding: '10px 12px' }}>
          <span style={{ color: 'var(--text-dim)' }}>preview&nbsp;</span>
          const worktree = await hive.<span style={{ color: 'var(--vks-cyan)' }}>spawn</span>(node);  <span style={{ color: 'var(--console-success)' }}>{'// => ok'}</span>
        </div>
      </SettingsSection>

      <SettingsSection title="Task Execution" description="Default coding agent for new task attempts.">
        <SettingsRow label="Executor" htmlFor="s-exec" helper="Availability is checked when an attempt starts.">
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <Select id="s-exec" value={g.executor} onValueChange={(v) => set('executor', v)}
              options={sel([{ value: 'CLAUDE_CODE', label: 'Claude Code' }, { value: 'CODEX', label: 'Codex' }, { value: 'OPENCODE', label: 'OpenCode' }, { value: 'GEMINI', label: 'Gemini' }])} />
            <Select value={g.variant} onValueChange={(v) => set('variant', v)}
              options={sel([{ value: 'DEFAULT', label: 'Default' }, { value: 'PLAN', label: 'Plan' }, { value: 'ROUTER', label: 'Router' }])} />
          </div>
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Editor" description="Opens worktrees in your local IDE.">
        <SettingsRow label="Preferred editor" htmlFor="s-editor" helper="Used by “Open in IDE”.">
          <Select id="s-editor" value={g.editorType} onValueChange={(v) => set('editorType', v)}
            options={sel([{ value: 'VS_CODE', label: 'VS Code' }, { value: 'CURSOR', label: 'Cursor' }, { value: 'ZED', label: 'Zed' }, { value: 'WINDSURF', label: 'Windsurf' }, { value: 'INTELLIJ', label: 'IntelliJ' }, { value: 'CUSTOM', label: 'Custom' }])} />
        </SettingsRow>
        {g.editorType === 'CUSTOM' && (
          <SettingsRow nested label="Custom command" htmlFor="s-cmd" helper="Receives the worktree path as its final argument.">
            <Input id="s-cmd" mono value={g.customCommand} placeholder="e.g. code --wait" onChange={(e) => set('customCommand', e.target.value)} />
          </SettingsRow>
        )}
        {remoteCapable && (
          <SettingsRow nested label="Remote SSH host" htmlFor="s-ssh" helper="Open worktrees over Remote-SSH. Leave blank for local.">
            <Input id="s-ssh" mono value={g.remoteHost} placeholder="e.g. justX.raverx.net" onChange={(e) => set('remoteHost', e.target.value)} />
          </SettingsRow>
        )}
        {remoteCapable && g.remoteHost && (
          <SettingsRow nested label="Remote SSH user" htmlFor="s-sshuser">
            <Input id="s-sshuser" mono value={g.remoteUser} placeholder="e.g. david" onChange={(e) => set('remoteUser', e.target.value)} />
          </SettingsRow>
        )}
      </SettingsSection>

      <SettingsSection title="Terminal" description="Appearance of the embedded log/terminal viewer."
        footer={<Button variant="outline" size="sm" onClick={() => patch('general', { terminalFontSize: '14', cursorBlink: true })}>Reset to defaults</Button>}>
        <SettingsRow label="Font size" htmlFor="s-tsize">
          <Select id="s-tsize" value={g.terminalFontSize} onValueChange={(v) => set('terminalFontSize', v)}
            options={sel([{ value: '10', label: '10px' }, { value: '12', label: '12px' }, { value: '14', label: '14px (default)' }, { value: '16', label: '16px' }, { value: '18', label: '18px' }, { value: '20', label: '20px' }])} />
        </SettingsRow>
        <SettingsRow inline label="Cursor blink" htmlFor="s-blink" helper="Blink the terminal cursor.">
          <Checkbox id="s-blink" checked={g.cursorBlink} onCheckedChange={(v) => set('cursorBlink', v)} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Timestamps" description="How times are shown in logs and activity.">
        <SettingsRow label="Timezone" htmlFor="s-tz">
          <Select id="s-tz" value={g.timezone} onValueChange={(v) => set('timezone', v)}
            options={sel([{ value: 'LOCAL', label: 'Local time' }, { value: 'UTC', label: 'UTC' }, { value: 'America/Los_Angeles', label: 'America/Los_Angeles (PST/PDT)' }, { value: 'Europe/London', label: 'Europe/London (GMT/BST)' }, { value: 'Australia/Sydney', label: 'Australia/Sydney (AEST/AEDT)' }, { value: 'Asia/Tokyo', label: 'Asia/Tokyo (JST)' }])} />
        </SettingsRow>
        <SettingsRow inline label="Token-level timestamps" htmlFor="s-tokents" helper="Prefix each streamed token with a timestamp.">
          <Switch id="s-tokents" checked={g.tokenTs} onCheckedChange={(v) => set('tokenTs', v)} />
        </SettingsRow>
        {g.tokenTs && (
          <SettingsRow nested label="Timestamp format" htmlFor="s-tokenfmt" helper="date-fns tokens. Applied to the token prefix.">
            <Input id="s-tokenfmt" mono value={g.tokenTsFormat} onChange={(e) => set('tokenTsFormat', e.target.value)} />
          </SettingsRow>
        )}
      </SettingsSection>

      <SettingsSection title="Git" description="Defaults applied when agents create branches.">
        <SettingsRow label="Branch prefix" htmlFor="s-prefix" error={prefixError}
          helper={<>Prepended to generated branch names. Preview: <span className="vks-field__preview">{(g.branchPrefix ? g.branchPrefix + '/' : '') + 'feat-auth-callback'}</span></>}>
          <Input id="s-prefix" mono value={g.branchPrefix} placeholder="e.g. vk"
            aria-invalid={!!prefixError}
            style={prefixError ? { borderColor: 'var(--danger)' } : undefined}
            onChange={(e) => set('branchPrefix', e.target.value.trim())} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Notifications" description="Alerts when attempts finish or need review.">
        <SettingsRow inline label="Sound" htmlFor="s-sound" helper="Play a sound when an attempt completes.">
          <Checkbox id="s-sound" checked={g.soundEnabled} onCheckedChange={(v) => set('soundEnabled', v)} />
        </SettingsRow>
        {g.soundEnabled && (
          <SettingsRow nested label="Sound file" htmlFor="s-soundfile">
            <div style={{ display: 'flex', gap: 8 }}>
              <div style={{ flex: 1 }}>
                <Select id="s-soundfile" value={g.soundFile} onValueChange={(v) => set('soundFile', v)}
                  options={sel([{ value: 'ROOK', label: 'Rook' }, { value: 'ABSTRACT_SOUND', label: 'Abstract Sound' }, { value: 'COW_MOOING', label: 'Cow Mooing' }, { value: 'PHONE_VIBRATION', label: 'Phone Vibration' }])} />
              </div>
              <Button variant="outline" size="icon" title="Preview sound"><Icon d={SICONS.volume} size={16} /></Button>
            </div>
          </SettingsRow>
        )}
        <SettingsRow inline label="Push notifications" htmlFor="s-push" helper="Notify even when the tab is in the background.">
          <Checkbox id="s-push" checked={g.pushEnabled} onCheckedChange={(v) => set('pushEnabled', v)} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Performance" description="Tune how much history loads up front.">
        <SettingsRow label="Initial log load" htmlFor="s-load" helper="Older entries load on scroll.">
          <Select id="s-load" value={g.initialLoad} onValueChange={(v) => set('initialLoad', v)}
            options={sel([{ value: '50', label: '50 entries' }, { value: '100', label: '100 entries (default)' }, { value: '200', label: '200 entries' }, { value: '500', label: '500 entries' }])} />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

// ---- Layout + state --------------------------------------------------------
function SettingsView({ onClose, onApplyTheme }) {
  const { Button } = window.VKSwarmDesignSystem_067861;
  const Icon = window.Icon;
  const ICONS = window.ICONS;
  const bp = window.useBreakpoint();
  const compact = bp !== 'desktop';

  const [active, setActive] = useState('general');
  const [draft, setDraft] = useState(() => JSON.parse(JSON.stringify(DEFAULT_DRAFT)));
  const [saved, setSaved] = useState(() => JSON.parse(JSON.stringify(DEFAULT_DRAFT)));
  const [success, setSuccess] = useState(false);
  const successTimer = useRef(null);

  const patch = (panel, partial) => setDraft((d) => ({ ...d, [panel]: { ...d[panel], ...partial } }));
  const dirty = JSON.stringify(draft) !== JSON.stringify(saved);

  useEffect(() => {
    const onKey = (e) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  useEffect(() => () => successTimer.current && clearTimeout(successTimer.current), []);

  const save = () => {
    setSaved(JSON.parse(JSON.stringify(draft)));
    onApplyTheme && onApplyTheme(draft.general.theme);
    setSuccess(true);
    successTimer.current && clearTimeout(successTimer.current);
    successTimer.current = setTimeout(() => setSuccess(false), 2600);
  };
  const discard = () => setDraft(JSON.parse(JSON.stringify(saved)));

  const items = navItems(ICONS);
  const current = items.find((i) => i.id === active);
  const PANELS = window.VKS_PANELS || {};
  const Panel = active === 'general' ? GeneralPanel : PANELS[active];

  return (
    <div style={{ position: 'absolute', inset: 0, zIndex: 20, background: 'var(--background)', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 20px', height: 56, borderBottom: '1px solid var(--border)', flexShrink: 0 }}>
        <h1 style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)', fontWeight: 600, margin: 0 }}>Settings</h1>
        <button className="vks-btn vks-btn--ghost vks-btn--sm" onClick={onClose} style={{ gap: 6, border: '1px solid var(--border-strong)' }} title="Close settings">
          <Icon d={SICONS.x} size={15} />
          <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)' }}>ESC</span>
        </button>
      </div>

      <div style={{ flex: 1, display: 'flex', minHeight: 0, flexDirection: compact ? 'column' : 'row' }}>
        {/* Sidebar */}
        <aside style={{ width: compact ? 'auto' : 248, flexShrink: 0, borderRight: compact ? 0 : '1px solid var(--border)', borderBottom: compact ? '1px solid var(--border)' : 0, padding: compact ? '8px 12px' : '16px 12px', overflowX: compact ? 'auto' : 'visible', overflowY: compact ? 'hidden' : 'auto' }}>
          <nav style={{ display: 'flex', flexDirection: compact ? 'row' : 'column', gap: compact ? 6 : 2 }}>
            {items.map((it) => {
              const on = it.id === active;
              return (
                <button key={it.id} onClick={() => setActive(it.id)} style={{
                  display: 'flex', alignItems: 'flex-start', gap: 11, textAlign: 'left', width: compact ? 'auto' : '100%',
                  padding: compact ? '8px 12px' : '9px 11px', borderRadius: 'var(--radius-md)', cursor: 'pointer',
                  border: compact && on ? '1px solid var(--primary)' : '1px solid transparent',
                  background: on ? 'var(--surface-raised)' : 'transparent',
                  color: on ? 'var(--foreground)' : 'var(--text-muted)', whiteSpace: 'nowrap',
                  transition: 'background-color .15s ease, color .15s ease',
                }}
                onMouseEnter={(e) => { if (!on) e.currentTarget.style.background = 'var(--surface-card)'; }}
                onMouseLeave={(e) => { if (!on) e.currentTarget.style.background = 'transparent'; }}>
                  <span style={{ color: on ? 'var(--primary)' : 'var(--text-muted)', marginTop: 1, flexShrink: 0, display: 'flex' }}><Icon d={it.icon} size={16} /></span>
                  <span style={{ minWidth: 0 }}>
                    <span style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 500 }}>{it.name}</span>
                    {!compact && <span style={{ display: 'block', fontSize: 'var(--text-xs)', color: 'var(--text-dim)', marginTop: 1 }}>{it.desc}</span>}
                  </span>
                </button>
              );
            })}
          </nav>
        </aside>

        {/* Content */}
        <main style={{ flex: 1, minWidth: 0, overflowY: 'auto', padding: compact ? '20px 16px 8px' : '28px 32px 12px' }}>
          <div style={{ maxWidth: 720, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 20 }}>
            <div>
              <h2 style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-xl)', fontWeight: 600, margin: 0 }}>{current.name}</h2>
              <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-muted)', margin: '4px 0 0' }}>{current.desc}</p>
            </div>

            {success && <Alert variant="success" icon={<Icon d={SICONS.check} size={16} />}>Settings saved.</Alert>}

            {Panel ? <Panel draft={draft} patch={patch} /> : (
              <Alert variant="info" icon={<Icon d={SICONS.alert} size={16} />}>This panel isn’t wired yet.</Alert>
            )}

            {dirty && (
              <div className="vks-savebar">
                <span className="vks-savebar__hint"><span className="vks-savebar__dot" />Unsaved changes</span>
                <span style={{ display: 'flex', gap: 8 }}>
                  <Button variant="ghost" size="sm" onClick={discard}>Discard</Button>
                  <Button variant="primary" size="sm" onClick={save}>Save changes</Button>
                </span>
              </div>
            )}
            <div style={{ height: 8 }} />
          </div>
        </main>
      </div>
    </div>
  );
}

Object.assign(window, { SettingsView, SICONS });
