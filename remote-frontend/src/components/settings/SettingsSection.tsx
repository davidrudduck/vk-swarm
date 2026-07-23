import type { HTMLAttributes, ReactElement, ReactNode } from 'react';
import { cn } from '@/lib/utils';

export interface SettingsSectionProps extends Omit<HTMLAttributes<HTMLElement>, 'title'> {
  /** Card title (CardTitle). */
  title?: ReactNode;
  /** Muted description under the title. */
  description?: ReactNode;
  /** Optional leading icon (e.g. a lucide glyph) shown left of the title. */
  icon?: ReactNode;
  /** Optional footer content (actions like Reset), rendered in the card footer. */
  footer?: ReactNode;
  /** Extra class on the inner content wrapper. */
  contentClassName?: string;
}

/** Settings panel card: header (icon/title/description) + auto-stacked body. */
export function SettingsSection({
  title,
  description,
  icon,
  footer,
  className,
  contentClassName,
  children,
  ...props
}: SettingsSectionProps): ReactElement {
  return (
    <section className={cn('vks-card', className)} {...props}>
      <div className={cn('vks-card__header', icon && 'vks-settings__header')}>
        {icon && <span className="vks-settings__header-icon">{icon}</span>}
        <div style={{ minWidth: 0 }}>
          {title && <h3 className="vks-card__title">{title}</h3>}
          {description && <p className="vks-card__desc">{description}</p>}
        </div>
      </div>
      <div className={cn('vks-card__content', 'vks-settings__body', contentClassName)}>
        {children}
      </div>
      {footer && <div className="vks-card__footer">{footer}</div>}
    </section>
  );
}
