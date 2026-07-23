import { ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

export type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'destructive' | 'link';
export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg' | 'icon';

const SIZES: Record<ButtonSize, string> = {
  xs: 'vks-btn--xs',
  sm: 'vks-btn--sm',
  md: 'vks-btn--md',
  lg: 'vks-btn--lg',
  icon: 'vks-btn--icon',
};

const VARIANTS: Record<ButtonVariant, string> = {
  primary: 'vks-btn--primary',
  secondary: 'vks-btn--secondary',
  outline: 'vks-btn--outline',
  ghost: 'vks-btn--ghost',
  destructive: 'vks-btn--destructive',
  link: 'vks-btn--link',
};

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual style. `primary` is solid cyan; `ghost` for toolbar icons. @default 'primary' */
  variant?: ButtonVariant;
  /** Control height. @default 'md' */
  size?: ButtonSize;
}

/**
 * Primary action control for VK-Swarm. Solid cyan primary glows on hover;
 * ghost/icon variants populate dense toolbars.
 */
export function Button({ variant = 'primary', size = 'md', className, children, ...props }: ButtonProps) {
  const cls = cn('vks-btn', VARIANTS[variant] ?? VARIANTS.primary, SIZES[size] ?? SIZES.md, className);
  return (
    <button className={cls} {...props}>
      {children}
    </button>
  );
}
