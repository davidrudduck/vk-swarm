import React from 'react';

const VARIANTS = {
  default: 'vks-badge--default',
  secondary: 'vks-badge--secondary',
  destructive: 'vks-badge--destructive',
  outline: 'vks-badge--outline',
};

/** Small rounded-full label. Optional leading dot for counts / statuses. */
export function Badge({ variant = 'default', dot = false, className = '', children, ...props }) {
  const cls = ['vks-badge', VARIANTS[variant] || VARIANTS.default, className].filter(Boolean).join(' ');
  return (
    <span className={cls} {...props}>
      {dot && <span className="vks-badge__dot" />}
      {children}
    </span>
  );
}
