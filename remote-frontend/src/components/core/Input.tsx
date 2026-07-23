import { InputHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  /** Use the monospace family (branch names, tokens, paths). @default false */
  mono?: boolean;
}

/** Single-line text field with cyan focus ring. */
export function Input({ mono = false, className, ...props }: InputProps) {
  return <input className={cn('vks-input', mono && 'vks-input--mono', className)} {...props} />;
}
