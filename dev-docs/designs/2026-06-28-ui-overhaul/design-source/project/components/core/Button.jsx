import React from 'react';

const SIZES = { xs: 'vks-btn--xs', sm: 'vks-btn--sm', md: 'vks-btn--md', lg: 'vks-btn--lg', icon: 'vks-btn--icon' };
const VARIANTS = {
  primary: 'vks-btn--primary',
  secondary: 'vks-btn--secondary',
  outline: 'vks-btn--outline',
  ghost: 'vks-btn--ghost',
  destructive: 'vks-btn--destructive',
  link: 'vks-btn--link',
};

/**
 * VK-Swarm button. Mirrors the app's cva variants (default→primary, outline,
 * ghost, destructive, link) with compact terminal-dense sizing.
 */
export function Button({
  variant = 'primary',
  size = 'md',
  className = '',
  children,
  ...props
}) {
  const cls = ['vks-btn', VARIANTS[variant] || VARIANTS.primary, SIZES[size] || SIZES.md, className]
    .filter(Boolean)
    .join(' ');
  return (
    <button className={cls} {...props}>
      {children}
    </button>
  );
}
