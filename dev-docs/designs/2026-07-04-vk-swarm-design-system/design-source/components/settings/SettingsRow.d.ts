import * as React from 'react';

export interface SettingsRowProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Field label text. */
  label?: React.ReactNode;
  /** `htmlFor` linking the label to its control. */
  htmlFor?: string;
  /** Muted helper text under the control. */
  helper?: React.ReactNode;
  /** Error message (replaces helper, shown in the danger color). */
  error?: React.ReactNode;
  /** Inline layout: leading control + label/helper stack (for Checkbox/Switch). */
  inline?: boolean;
  /** Indent the row (a dependent field revealed under a toggle). */
  nested?: boolean;
  /** The control element; alternatively pass it as children. */
  control?: React.ReactNode;
}

/** Labelled settings control with helper/error text; stacked or inline. */
export function SettingsRow(props: SettingsRowProps): React.ReactElement;
