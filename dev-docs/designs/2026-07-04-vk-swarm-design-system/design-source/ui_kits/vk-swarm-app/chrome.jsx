// VK-Swarm UI kit — shared chrome (navbar) and small primitives built on the
// design-system bundle classes. Components register on window for sibling files.
const { useState } = React;

const Icon = ({ d, size = 16, stroke = 1.6, fill = 'none' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill={fill} stroke="currentColor"
    strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {d}
  </svg>
);

// Lucide-style stroke icons (24px grid, ~1.6px) — the product uses lucide-react.
const ICONS = {
  plus: <><path d="M12 5v14M5 12h14" /></>,
  search: <><circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" /></>,
  server: <><rect x="3" y="4" width="18" height="7" rx="1.5" /><rect x="3" y="13" width="18" height="7" rx="1.5" /><path d="M7 7.5h.01M7 16.5h.01" /></>,
  folder: <><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></>,
  activity: <><path d="M22 12h-4l-3 9L9 3l-3 9H2" /></>,
  settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-2.82 1.17V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 8 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 14H4.5a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 6 8.6a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 10 4.6h.09A1.65 1.65 0 0 0 11.4 3h.09a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 16 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 8z" /></>,
  menu: <><path d="M4 6h16M4 12h16M4 18h16" /></>,
  git: <><circle cx="6" cy="6" r="2.5" /><circle cx="6" cy="18" r="2.5" /><circle cx="18" cy="9" r="2.5" /><path d="M6 8.5v7M18 11.5c0 4-6 1.5-6 4.5" /></>,
  bolt: <><path d="M13 2L4.5 13H11l-1 9 8.5-11H12z" /></>,
  sun: <><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>,
  moon: <><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></>,
};

// Track viewport width so chrome adapts across mobile / tablet / desktop.
function useBreakpoint() {
  const get = () => (typeof window === 'undefined' ? 'desktop'
    : window.innerWidth < 640 ? 'mobile'
    : window.innerWidth < 1024 ? 'tablet' : 'desktop');
  const [bp, setBp] = useState(get);
  React.useEffect(() => {
    const on = () => setBp(get());
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return bp;
}

function Logo({ compact }) {
  return (
    <span className="vks-wordmark" style={{ fontSize: compact ? 16 : 18 }}>
      <span className="vk">VK</span><span className="swarm">{compact ? 'S' : '-SWARM'}</span>
    </span>
  );
}

function ThemeToggle({ theme, onToggle }) {
  return (
    <button className="vks-btn vks-btn--ghost vks-btn--icon" onClick={onToggle}
      title={theme === 'dark' ? 'Switch to light' : 'Switch to dark'}
      aria-label="Toggle theme" style={{ height: 34, width: 34 }}>
      <Icon d={theme === 'dark' ? ICONS.sun : ICONS.moon} size={16} />
    </button>
  );
}

function Navbar({ project, view, onView, onNewTask, theme, onToggleTheme, onOpenSettings }) {
  const bp = useBreakpoint();
  const mobile = bp === 'mobile';
  const tablet = bp === 'tablet';
  return (
    <header style={{ borderBottom: '1px solid var(--border)', background: 'var(--background)' }}>
      <div style={{ display: 'flex', alignItems: 'center', height: 48, padding: '0 12px', gap: mobile ? 8 : 12 }}>
        <Logo compact={mobile} />
        {!mobile && <div style={{ width: 1, height: 22, background: 'var(--border)', margin: '0 2px' }} />}
        <button className="vks-btn vks-btn--ghost vks-btn--sm" style={{ gap: 8, paddingLeft: 8 }}>
          <span style={{ color: 'var(--text-muted)' }}><Icon d={ICONS.folder} size={14} /></span>
          {!mobile && <span style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{project}</span>}
          <span style={{ color: 'var(--text-dim)' }}><Icon d={<path d="M6 9l6 6 6-6" />} size={12} /></span>
        </button>

        <div style={{ flex: 1 }} />

        {/* Search collapses to an icon button below desktop */}
        {bp === 'desktop' ? (
          <div style={{ position: 'relative', width: 260, maxWidth: '30vw' }}>
            <span style={{ position: 'absolute', left: 10, top: '50%', transform: 'translateY(-50%)', color: 'var(--text-dim)' }}>
              <Icon d={ICONS.search} size={14} />
            </span>
            <input className="vks-input" placeholder="Search tasks…" style={{ height: 34, paddingLeft: 32, fontSize: 'var(--text-sm)' }} />
          </div>
        ) : (
          <NavIcon icon={ICONS.search} title="Search" />
        )}

        {!mobile && (
          <button className="vks-btn vks-btn--ghost vks-btn--icon" title="Open in IDE" style={{ height: 34, width: 34 }}>
            <Icon d={ICONS.bolt} size={16} />
          </button>
        )}
        <button className="vks-btn vks-btn--primary vks-btn--sm" onClick={onNewTask} style={{ gap: 6 }}>
          <Icon d={ICONS.plus} size={14} /> {!mobile && 'Task'}
        </button>
        {!mobile && <div style={{ width: 1, height: 22, background: 'var(--border)', margin: '0 2px' }} />}
        <ThemeToggle theme={theme} onToggle={onToggleTheme} />
        {!tablet && !mobile && <NavIcon icon={ICONS.activity} title="Activity" />}
        {!mobile && <NavIcon icon={ICONS.settings} title="Settings" onClick={onOpenSettings} />}
        <NavIcon icon={ICONS.menu} title="Menu" />
      </div>
      <nav style={{ display: 'flex', gap: 2, padding: '0 12px', overflowX: 'auto' }}>
        <NavTab active={view === 'board'} onClick={() => onView('board')} icon={ICONS.folder} label="Board" />
        <NavTab active={view === 'nodes'} onClick={() => onView('nodes')} icon={ICONS.server} label="Nodes" />
        <NavTab active={view === 'processes'} onClick={() => onView('processes')} icon={ICONS.activity} label="Processes" />
      </nav>
    </header>
  );
}

function NavIcon({ icon, title, onClick }) {
  return (
    <button className="vks-btn vks-btn--ghost vks-btn--icon" title={title} onClick={onClick} style={{ height: 34, width: 34 }}>
      <Icon d={icon} size={16} />
    </button>
  );
}

function NavTab({ active, onClick, icon, label }) {
  return (
    <button onClick={onClick} style={{
      display: 'flex', alignItems: 'center', gap: 7, padding: '9px 12px', background: 'transparent',
      border: 0, borderBottom: `2px solid ${active ? 'var(--primary)' : 'transparent'}`,
      color: active ? 'var(--foreground)' : 'var(--text-muted)', fontFamily: 'var(--font-ui)',
      fontSize: 'var(--text-sm)', fontWeight: 500, cursor: 'pointer', marginBottom: -1, whiteSpace: 'nowrap',
    }}>
      <Icon d={icon} size={14} /> {label}
    </button>
  );
}

Object.assign(window, { Icon, ICONS, Navbar, Logo, useBreakpoint, ThemeToggle });
