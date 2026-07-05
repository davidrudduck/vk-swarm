import * as React from 'react';

export interface CheckboxProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
}

/** Square checkbox with cyan fill + check glyph when selected. */
export function Checkbox(props: CheckboxProps): React.ReactElement;
