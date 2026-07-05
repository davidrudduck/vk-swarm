import * as React from 'react';

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
export function Switch(props: SwitchProps): React.ReactElement;
