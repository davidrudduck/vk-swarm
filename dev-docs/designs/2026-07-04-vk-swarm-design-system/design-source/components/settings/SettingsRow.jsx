import React from 'react';

/**
 * A single labelled setting: a label, its control, and helper/error text.
 *
 * - Default (stacked): label on top, control, then helper below — used for
 *   Select / Input rows.
 * - `inline`: a leading control (Checkbox / Switch) followed by a label +
 *   helper stack to its right — used for boolean toggles.
 *
 * The control is passed as `control` or as children.
 */
export function SettingsRow({
  label,
  htmlFor,
  helper,
  error,
  inline = false,
  nested = false,
  control,
  className = '',
  children,
  ...props
}) {
  const body = control ?? children;
  const cls = [
    'vks-field',
    inline && 'vks-field--inline',
    nested && 'vks-field--nested',
    className,
  ].filter(Boolean).join(' ');

  if (inline) {
    return (
      <div className={cls} {...props}>
        {body}
        <div className="vks-field__body">
          {label && <label htmlFor={htmlFor} className="vks-field__label">{label}</label>}
          {helper && <p className="vks-field__helper">{helper}</p>}
          {error && <p className="vks-field__error">{error}</p>}
        </div>
      </div>
    );
  }

  return (
    <div className={cls} {...props}>
      {label && <label htmlFor={htmlFor} className="vks-field__label">{label}</label>}
      {body}
      {error && <p className="vks-field__error">{error}</p>}
      {helper && !error && <p className="vks-field__helper">{helper}</p>}
    </div>
  );
}
