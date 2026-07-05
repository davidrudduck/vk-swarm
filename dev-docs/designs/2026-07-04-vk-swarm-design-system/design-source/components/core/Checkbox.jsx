import React from 'react';

/** Controlled or uncontrolled checkbox. */
export function Checkbox({ checked, defaultChecked = false, onCheckedChange, disabled = false, className = '', ...props }) {
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(defaultChecked);
  const on = isControlled ? checked : internal;
  const toggle = () => {
    if (disabled) return;
    if (!isControlled) setInternal(!on);
    onCheckedChange && onCheckedChange(!on);
  };
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={on}
      data-checked={on}
      disabled={disabled}
      onClick={toggle}
      className={['vks-checkbox', className].filter(Boolean).join(' ')}
      {...props}
    >
      <svg width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
        <path d="M2.5 6.5l2.5 2.5 4.5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </button>
  );
}
