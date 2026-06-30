import * as React from 'react';

export interface LoaderProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** sm | md | lg, or a pixel size. @default 'md' */
  size?: 'sm' | 'md' | 'lg' | number;
}

/** Cyan-topped spinner for in-progress states. */
export function Loader(props: LoaderProps): React.ReactElement;
