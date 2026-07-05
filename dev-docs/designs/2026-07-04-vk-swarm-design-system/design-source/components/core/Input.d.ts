import * as React from 'react';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  /** Use the monospace family (branch names, tokens, paths). @default false */
  mono?: boolean;
}

/** Single-line text field with cyan focus ring. */
export function Input(props: InputProps): React.ReactElement;
