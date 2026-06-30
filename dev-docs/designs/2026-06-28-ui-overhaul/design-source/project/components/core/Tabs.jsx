import React from 'react';

/**
 * Segmented tab control.
 * @param {{value:string,label:React.ReactNode}[]} tabs
 */
export function Tabs({ tabs = [], value, defaultValue, onValueChange, className = '' }) {
  const isControlled = value !== undefined;
  const [internal, setInternal] = React.useState(defaultValue ?? (tabs[0] && tabs[0].value));
  const active = isControlled ? value : internal;
  const select = (v) => {
    if (!isControlled) setInternal(v);
    onValueChange && onValueChange(v);
  };
  return (
    <div className={['vks-tabs__list', className].filter(Boolean).join(' ')} role="tablist">
      {tabs.map((t) => (
        <button
          key={t.value}
          role="tab"
          aria-selected={active === t.value}
          data-active={active === t.value}
          className="vks-tabs__trigger"
          onClick={() => select(t.value)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
