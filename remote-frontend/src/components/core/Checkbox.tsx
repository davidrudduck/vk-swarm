import { useState } from 'react';
import { cn } from '@/lib/utils';

export interface CheckboxProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
}

/** Square checkbox with cyan fill + check glyph when selected. */
export function Checkbox({
  checked,
  defaultChecked = false,
  onCheckedChange,
  disabled = false,
  className,
  ...props
}: CheckboxProps) {
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
      role="checkbox"
      aria-checked={on}
      data-checked={on}
      disabled={disabled}
      onClick={toggle}
      className={cn('vks-checkbox', className)}
      {...props}
    >
      <svg width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
        <path
          d="M2.5 6.5l2.5 2.5 4.5-5"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </button>
  );
}
