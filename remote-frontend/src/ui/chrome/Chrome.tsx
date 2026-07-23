import type { ReactElement } from 'react';
import { Icon, ICONS } from './icons';
import { useBreakpoint } from './useBreakpoint';

export function Logo({ compact }: { compact?: boolean }) {
  return (
    <span className="vks-wordmark" style={{ fontSize: compact ? 16 : 18 }}>
      <span className="vk">VK</span>
      <span className="swarm">{compact ? 'S' : '-SWARM'}</span>
    </span>
  );
}

export function ThemeToggle({
  theme,
  onToggle,
}: {
  theme: 'dark' | 'light';
  onToggle: () => void;
}) {
  return (
    <button
      className="vks-btn vks-btn--ghost vks-btn--icon"
      onClick={onToggle}
      title={theme === 'dark' ? 'Switch to light' : 'Switch to dark'}
      aria-label="Toggle theme"
      style={{ height: 34, width: 34 }}
    >
      <Icon d={theme === 'dark' ? ICONS.sun : ICONS.moon} size={16} />
    </button>
  );
}

export function NavIcon({
  icon,
  title,
  onClick,
  disabled,
}: {
  icon: ReactElement;
  title: string;
  onClick?: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      className="vks-btn vks-btn--ghost vks-btn--icon"
      title={title}
      onClick={onClick}
      disabled={disabled}
      style={{ height: 34, width: 34 }}
    >
      <Icon d={icon} size={16} />
    </button>
  );
}

export function NavTab({
  active,
  onClick,
  icon,
  label,
}: {
  active: boolean;
  onClick: () => void;
  icon: ReactElement;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 7,
        padding: '9px 12px',
        background: 'transparent',
        border: 0,
        borderBottom: `2px solid ${active ? 'var(--primary)' : 'transparent'}`,
        color: active ? 'var(--foreground)' : 'var(--text-muted)',
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--text-sm)',
        fontWeight: 500,
        cursor: 'pointer',
        marginBottom: -1,
        whiteSpace: 'nowrap',
      }}
    >
      <Icon d={icon} size={14} /> {label}
    </button>
  );
}

export interface NavbarProps {
  project: string;
  view: 'board' | 'nodes' | 'processes';
  onView: (v: 'board' | 'nodes' | 'processes') => void;
  /** Omit to render the "New Task" control disabled (no backing hive API yet). */
  onNewTask?: () => void;
  theme: 'dark' | 'light';
  onToggleTheme: () => void;
  /** Omit to render the "Settings" control disabled (no backing hive API yet). */
  onOpenSettings?: () => void;
}

/** Tooltip shown on controls whose backing hive API is not implemented yet. */
const NOT_WIRED_TITLE = 'Not yet wired to the hive API';

export function Navbar({
  project,
  view,
  onView,
  onNewTask,
  theme,
  onToggleTheme,
  onOpenSettings,
}: NavbarProps) {
  const bp = useBreakpoint();
  const mobile = bp === 'mobile';
  const tablet = bp === 'tablet';
  return (
    <header style={{ borderBottom: '1px solid var(--border)', background: 'var(--background)' }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          height: 48,
          padding: '0 12px',
          gap: mobile ? 8 : 12,
        }}
      >
        <Logo compact={mobile} />
        {!mobile && (
          <div style={{ width: 1, height: 22, background: 'var(--border)', margin: '0 2px' }} />
        )}
        <button
          className="vks-btn vks-btn--ghost vks-btn--sm"
          disabled
          title={NOT_WIRED_TITLE}
          style={{ gap: 8, paddingLeft: 8 }}
        >
          <span style={{ color: 'var(--text-muted)' }}>
            <Icon d={ICONS.folder} size={14} />
          </span>
          {!mobile && (
            <span style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{project}</span>
          )}
          <span style={{ color: 'var(--text-dim)' }}>
            <Icon d={<path d="M6 9l6 6 6-6" />} size={12} />
          </span>
        </button>

        <div style={{ flex: 1 }} />

        {/* Search collapses to an icon button below desktop */}
        {bp === 'desktop' ? (
          <div style={{ position: 'relative', width: 260, maxWidth: '30vw' }}>
            <span
              style={{
                position: 'absolute',
                left: 10,
                top: '50%',
                transform: 'translateY(-50%)',
                color: 'var(--text-dim)',
              }}
            >
              <Icon d={ICONS.search} size={14} />
            </span>
            <input
              className="vks-input"
              placeholder="Search tasks…"
              style={{ height: 34, paddingLeft: 32, fontSize: 'var(--text-sm)' }}
            />
          </div>
        ) : (
          <NavIcon icon={ICONS.search} title="Search" />
        )}

        {!mobile && (
          <button
            className="vks-btn vks-btn--ghost vks-btn--icon"
            disabled
            title={`Open in IDE — ${NOT_WIRED_TITLE}`}
            style={{ height: 34, width: 34 }}
          >
            <Icon d={ICONS.bolt} size={16} />
          </button>
        )}
        <button
          className="vks-btn vks-btn--primary vks-btn--sm"
          onClick={onNewTask}
          disabled={!onNewTask}
          title={onNewTask ? undefined : NOT_WIRED_TITLE}
          style={{ gap: 6 }}
        >
          <Icon d={ICONS.plus} size={14} /> {!mobile && 'Task'}
        </button>
        {!mobile && (
          <div style={{ width: 1, height: 22, background: 'var(--border)', margin: '0 2px' }} />
        )}
        <ThemeToggle theme={theme} onToggle={onToggleTheme} />
        {!tablet && !mobile && <NavIcon icon={ICONS.activity} title="Activity" />}
        {!mobile && (
          <NavIcon
            icon={ICONS.settings}
            title={onOpenSettings ? 'Settings' : `Settings — ${NOT_WIRED_TITLE}`}
            onClick={onOpenSettings}
            disabled={!onOpenSettings}
          />
        )}
        <NavIcon icon={ICONS.menu} title="Menu" />
      </div>
      <nav style={{ display: 'flex', gap: 2, padding: '0 12px', overflowX: 'auto' }}>
        <NavTab
          active={view === 'board'}
          onClick={() => onView('board')}
          icon={ICONS.folder}
          label="Board"
        />
        <NavTab
          active={view === 'nodes'}
          onClick={() => onView('nodes')}
          icon={ICONS.server}
          label="Nodes"
        />
        <NavTab
          active={view === 'processes'}
          onClick={() => onView('processes')}
          icon={ICONS.activity}
          label="Processes"
        />
      </nav>
    </header>
  );
}
