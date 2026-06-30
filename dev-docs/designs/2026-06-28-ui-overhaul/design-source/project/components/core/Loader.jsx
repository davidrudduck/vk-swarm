import React from 'react';

const SIZES = { sm: 14, md: 18, lg: 28 };

/** Spinner. `size` is sm|md|lg or a pixel number. */
export function Loader({ size = 'md', className = '', style, ...props }) {
  const px = typeof size === 'number' ? size : SIZES[size] || SIZES.md;
  return (
    <span
      className={['vks-loader', className].filter(Boolean).join(' ')}
      style={{ width: px, height: px, ...style }}
      role="status"
      aria-label="Loading"
      {...props}
    />
  );
}
