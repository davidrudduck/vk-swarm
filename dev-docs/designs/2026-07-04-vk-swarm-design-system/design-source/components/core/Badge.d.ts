import * as React from 'react';

export type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** @default 'default' */
  variant?: BadgeVariant;
  /** Show a leading dot (counts, presence). @default false */
  dot?: boolean;
}

/** Compact pill for counts, labels and metadata. */
export function Badge(props: BadgeProps): React.ReactElement;
