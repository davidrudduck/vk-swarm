import type { HTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

export interface LoaderProps extends HTMLAttributes<HTMLSpanElement> {
  /** sm | md | lg, or a pixel size. @default 'md' */
  size?: 'sm' | 'md' | 'lg' | number;
}

const SIZES: Record<'sm' | 'md' | 'lg', number> = { sm: 14, md: 18, lg: 28 };

/** Cyan-topped spinner for in-progress states. */
export function Loader({ size = 'md', className, style, ...props }: LoaderProps) {
  const px = typeof size === 'number' ? size : (SIZES[size] ?? SIZES.md);
  return (
    <span
      className={cn('vks-loader', className)}
      style={{ width: px, height: px, ...style }}
      role="status"
      aria-label="Loading"
      {...props}
    />
  );
}
