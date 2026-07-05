import * as React from 'react';

export interface SettingsSectionProps extends React.HTMLAttributes<HTMLElement> {
  /** Card title (CardTitle). */
  title?: React.ReactNode;
  /** Muted description under the title. */
  description?: React.ReactNode;
  /** Optional leading icon (e.g. a lucide glyph) shown left of the title. */
  icon?: React.ReactNode;
  /** Optional footer content (actions like Reset), rendered in the card footer. */
  footer?: React.ReactNode;
  /** Extra class on the inner content wrapper. */
  contentClassName?: string;
}

/** Settings panel card: header (icon/title/description) + auto-stacked body. */
export function SettingsSection(props: SettingsSectionProps): React.ReactElement;
