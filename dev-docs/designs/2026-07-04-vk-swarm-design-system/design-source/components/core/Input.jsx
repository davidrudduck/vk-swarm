import React from 'react';

/** Text input on `--input` surface. Pass `mono` for code/branch fields. */
export function Input({ mono = false, className = '', ...props }) {
  const cls = ['vks-input', mono && 'vks-input--mono', className].filter(Boolean).join(' ');
  return <input className={cls} {...props} />;
}
