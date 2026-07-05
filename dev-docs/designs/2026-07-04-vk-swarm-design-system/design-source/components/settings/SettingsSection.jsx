import React from 'react';

/**
 * A settings panel: a Card with a header (title, optional description and
 * leading icon) whose body vertically stacks its child fields with even gaps.
 * Pass a `footer` for actions like Reset. Mirrors the app's Card + CardHeader
 * pattern used across every settings page.
 */
export function SettingsSection({
  title,
  description,
  icon,
  footer,
  className = '',
  contentClassName = '',
  children,
  ...props
}) {
  return (
    <section className={['vks-card', className].filter(Boolean).join(' ')} {...props}>
      <div className={['vks-card__header', icon && 'vks-settings__header'].filter(Boolean).join(' ')}>
        {icon && <span className="vks-settings__header-icon">{icon}</span>}
        <div style={{ minWidth: 0 }}>
          {title && <h3 className="vks-card__title">{title}</h3>}
          {description && <p className="vks-card__desc">{description}</p>}
        </div>
      </div>
      <div className={['vks-card__content', 'vks-settings__body', contentClassName].filter(Boolean).join(' ')}>
        {children}
      </div>
      {footer && <div className="vks-card__footer">{footer}</div>}
    </section>
  );
}
