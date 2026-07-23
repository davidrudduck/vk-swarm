import type { HTMLAttributes, ReactElement, ReactNode } from 'react';
import { cn } from '@/lib/utils';

export interface SettingsRowProps extends HTMLAttributes<HTMLDivElement> {
  /** Field label text. */
  label?: ReactNode;
  /** `htmlFor` linking the label to its control. */
  htmlFor?: string;
  /** Muted helper text under the control. */
  helper?: ReactNode;
  /** Error message (replaces helper, shown in the danger color). */
  error?: ReactNode;
  /** Inline layout: leading control + label/helper stack (for Checkbox/Switch). */
  inline?: boolean;
  /** Indent the row (a dependent field revealed under a toggle). */
  nested?: boolean;
  /** The control element; alternatively pass it as children. */
  control?: ReactNode;
}

/** Labelled settings control with helper/error text; stacked or inline. */
export function SettingsRow({
  label,
  htmlFor,
  helper,
  error,
  inline = false,
  nested = false,
  control,
  className,
  children,
  ...props
}: SettingsRowProps): ReactElement {
  const body = control ?? children;
  const cls = cn('vks-field', inline && 'vks-field--inline', nested && 'vks-field--nested', className);

  if (inline) {
    return (
      <div className={cls} {...props}>
        {body}
        <div className="vks-field__body">
          {label && (
            <label htmlFor={htmlFor} className="vks-field__label">
              {label}
            </label>
          )}
          {helper && <p className="vks-field__helper">{helper}</p>}
          {error && <p className="vks-field__error">{error}</p>}
        </div>
      </div>
    );
  }

  return (
    <div className={cls} {...props}>
      {label && (
        <label htmlFor={htmlFor} className="vks-field__label">
          {label}
        </label>
      )}
      {body}
      {error && <p className="vks-field__error">{error}</p>}
      {helper && !error && <p className="vks-field__helper">{helper}</p>}
    </div>
  );
}
