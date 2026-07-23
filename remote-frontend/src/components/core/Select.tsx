import { useState } from 'react';
import type { ChangeEvent, SelectHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps
  extends Omit<SelectHTMLAttributes<HTMLSelectElement>, 'onChange' | 'value' | 'defaultValue'> {
  options: SelectOption[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  disabled?: boolean;
  className?: string;
}

/** Styled native dropdown (agent picker, branch picker, config). */
export function Select({
  options = [],
  value,
  defaultValue,
  onValueChange,
  disabled = false,
  className,
  ...props
}: SelectProps) {
  const isControlled = value !== undefined;
  const [internal, setInternal] = useState(defaultValue ?? options[0]?.value);
  const v = isControlled ? value : internal;

  const change = (e: ChangeEvent<HTMLSelectElement>) => {
    if (!isControlled) setInternal(e.target.value);
    onValueChange?.(e.target.value);
  };

  return (
    <div className={cn('vks-select', className)}>
      <select value={v} onChange={change} disabled={disabled} {...props}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      <span className="vks-select__chevron" aria-hidden="true">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path
            d="M3 4.5L6 7.5L9 4.5"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    </div>
  );
}
