import * as React from 'react';

export type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'destructive' | 'link';
export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg' | 'icon';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual style. `primary` is solid cyan; `ghost` for toolbar icons. @default 'primary' */
  variant?: ButtonVariant;
  /** Control height. @default 'md' */
  size?: ButtonSize;
}

/**
 * Primary action control for VK-Swarm. Solid cyan primary glows on hover;
 * ghost/icon variants populate dense toolbars.
 *
 * @startingPoint section="Core" subtitle="Buttons in every variant & size" viewport="700x160"
 */
export function Button(props: ButtonProps): React.ReactElement;
