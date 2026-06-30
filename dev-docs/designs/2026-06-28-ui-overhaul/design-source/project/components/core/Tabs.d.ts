import * as React from 'react';

export interface TabItem {
  value: string;
  label: React.ReactNode;
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
export function Tabs(props: TabsProps): React.ReactElement;
