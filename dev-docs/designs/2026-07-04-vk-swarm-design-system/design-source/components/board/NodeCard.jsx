import React from 'react';

const OS_GLYPH = {
  mac: <path d="M12 4.5c.4-1 .3-2 .2-2.3-.9.1-1.9.7-2.5 1.4-.5.6-.9 1.6-.8 2.5 1 .1 2-.5 2.6-1.2.3-.1.4-.3.5-.4zM14.8 11.4c-.5 1.1-.7 1.6-1.4 2.6-.9 1.3-2.2 3-3.8 3-1.4 0-1.8-.9-3.7-.9s-2.3.9-3.7.9c-1.6 0-2.8-1.5-3.7-2.8C-.8 12.1-.3 9 1 7.2c.9-1.3 2.2-2 3.5-2 1.4 0 2.2.9 3.4.9 1.1 0 1.8-.9 3.4-.9 1.2 0 2.5.6 3.4 1.7-3 1.6-2.5 5.9-.3 4.4z" transform="translate(1 0)" />,
  linux: <path d="M8 1c-1.8 0-2.6 1.6-2.6 3.4 0 1 .2 1.7.2 2.6 0 1-.9 1.8-1.6 3-.7 1.2-1.4 2.4-1.4 3.6 0 .9.5 1.4 1.3 1.4.6 0 1-.3 1.4-.3.3 0 .5.2.8.4.5.3 1.2.5 2 .5s1.5-.2 2-.5c.3-.2.5-.4.8-.4.4 0 .8.3 1.4.3.8 0 1.3-.5 1.3-1.4 0-1.2-.7-2.4-1.4-3.6-.7-1.2-1.6-2-1.6-3 0-.9.2-1.6.2-2.6C10.6 2.6 9.8 1 8 1z" />,
  windows: <path d="M1 2.8l5.7-.8v5.5H1V2.8zm6.4-.9L15 1v6.5H7.4V1.9zM1 8.2h5.7v5.5L1 13V8.2zm6.4 0H15V15l-7.6-1V8.2z" />,
};

/** Swarm node row: OS glyph, name, status pulse, optional meta. */
export function NodeCard({ name, os = 'linux', online = true, meta, right, className = '', ...props }) {
  const cls = ['vks-node', className].filter(Boolean).join(' ');
  return (
    <div className={cls} {...props}>
      <div className="vks-node__os">
        <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">{OS_GLYPH[os] || OS_GLYPH.linux}</svg>
      </div>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span className="vks-node__name">{name}</span>
          <span className={online ? 'vks-node__pulse' : 'vks-node__pulse vks-node__pulse--offline'} />
        </div>
        {meta && <div style={{ fontSize: 'var(--text-sm)', color: 'var(--text-muted)', marginTop: 2 }}>{meta}</div>}
      </div>
      {right}
    </div>
  );
}
