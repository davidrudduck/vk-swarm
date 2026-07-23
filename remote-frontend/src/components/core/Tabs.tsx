import { useState } from 'react';
import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

export interface TabItem {
  value: string;
  label: ReactNode;
}

export interface TabsProps {
  tabs: TabItem[];
  /** Controlled active value. */
  value?: string;
  /** Initial value when uncontrolled (defaults to first tab). */
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  className?: string;
}

/** Segmented control for switching views (e.g. Diff / Logs / Processes). */
export function Tabs({ tabs = [], value, defaultValue, onValueChange, className }: TabsProps) {
  const isControlled = value !== undefined;
  const [internal, setInternal] = useState(defaultValue ?? tabs[0]?.value);
  const active = isControlled ? value : internal;

  const select = (v: string) => {
    if (!isControlled) setInternal(v);
    onValueChange?.(v);
  };

  return (
    <div className={cn('vks-tabs__list', className)} role="tablist">
      {tabs.map((t) => (
        <button
          key={t.value}
          type="button"
          role="tab"
          aria-selected={active === t.value}
          data-active={active === t.value}
          className="vks-tabs__trigger"
          onClick={() => select(t.value)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
