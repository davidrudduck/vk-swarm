import React from 'react';

/** Native select styled for the Midnight Terminal theme. */
export function Select({ options = [], value, defaultValue, onValueChange, disabled = false, className = '', ...props }) {
  const isControlled = value !== undefined;
  const [internal, setInternal] = React.useState(defaultValue ?? (options[0] && options[0].value));
  const v = isControlled ? value : internal;
  const change = (e) => {
    if (!isControlled) setInternal(e.target.value);
    onValueChange && onValueChange(e.target.value);
  };
  return (
    <div className={['vks-select', className].filter(Boolean).join(' ')}>
      <select value={v} onChange={change} disabled={disabled} {...props}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
      <span className="vks-select__chevron" aria-hidden="true">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none"><path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/></svg>
      </span>
    </div>
  );
}
