import { HTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

export type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline';

const VARIANTS: Record<BadgeVariant, string> = {
  default: 'vks-badge--default',
  secondary: 'vks-badge--secondary',
  destructive: 'vks-badge--destructive',
  outline: 'vks-badge--outline',
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  /** @default 'default' */
  variant?: BadgeVariant;
  /** Show a leading dot (counts, presence). @default false */
  dot?: boolean;
}

/** Compact pill for counts, labels and metadata. */
export function Badge({ variant = 'default', dot = false, className, children, ...props }: BadgeProps) {
  const cls = cn('vks-badge', VARIANTS[variant] ?? VARIANTS.default, className);
  return (
    <span className={cls} {...props}>
      {dot && <span className="vks-badge__dot" />}
      {children}
    </span>
  );
}
