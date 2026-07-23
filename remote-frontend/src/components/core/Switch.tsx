import { useState } from 'react';
import { cn } from '@/lib/utils';

export interface SwitchProps {
  /** Controlled on/off. */
  checked?: boolean;
  /** Initial state when uncontrolled. @default false */
  defaultChecked?: boolean;
  /** Fired with the next value on toggle. */
  onCheckedChange?: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
}

/** Cyan toggle switch for settings & feature flags. */
export function Switch({
  checked,
  defaultChecked = false,
  onCheckedChange,
  disabled = false,
  className,
  ...props
}: SwitchProps) {
  const isControlled = checked !== undefined;
  const [internal, setInternal] = useState(defaultChecked);
  const on = isControlled ? checked : internal;

  const toggle = () => {
    if (disabled) return;
    if (!isControlled) setInternal(!on);
    onCheckedChange?.(!on);
  };

  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      data-checked={on}
      disabled={disabled}
      onClick={toggle}
      className={cn('vks-switch', className)}
      {...props}
    >
      <span className="vks-switch__thumb" />
    </button>
  );
}
