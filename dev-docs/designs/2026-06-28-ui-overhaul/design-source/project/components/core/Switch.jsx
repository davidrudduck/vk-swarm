import React from 'react';

/** Controlled or uncontrolled toggle switch. */
export function Switch({ checked, defaultChecked = false, onCheckedChange, disabled = false, className = '', ...props }) {
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
      role="switch"
      aria-checked={on}
      data-checked={on}
      disabled={disabled}
      onClick={toggle}
      className={['vks-switch', className].filter(Boolean).join(' ')}
      {...props}
    >
      <span className="vks-switch__thumb" />
    </button>
  );
}
