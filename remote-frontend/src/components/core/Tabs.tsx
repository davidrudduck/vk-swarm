import { useRef, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
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
  const triggerRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const select = (v: string) => {
    if (!isControlled) setInternal(v);
    onValueChange?.(v);
  };

  const focusTab = (index: number) => {
    const t = tabs[index];
    if (!t) return;
    triggerRefs.current[index]?.focus();
    select(t.value);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLButtonElement>, index: number) => {
    let next: number;
    switch (e.key) {
      case 'ArrowRight':
        next = (index + 1) % tabs.length;
        break;
      case 'ArrowLeft':
        next = (index - 1 + tabs.length) % tabs.length;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = tabs.length - 1;
        break;
      default:
        return;
    }
    e.preventDefault();
    focusTab(next);
  };

  return (
    <div className={cn('vks-tabs__list', className)} role="tablist">
      {tabs.map((t, i) => {
        const isActive = active === t.value;
        return (
          <button
            key={t.value}
            ref={(el) => {
              triggerRefs.current[i] = el;
            }}
            type="button"
            role="tab"
            aria-selected={isActive}
            data-active={isActive}
            tabIndex={isActive ? 0 : -1}
            className="vks-tabs__trigger"
            onClick={() => select(t.value)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {t.label}
          </button>
        );
      })}
    </div>
  );
}
