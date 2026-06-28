import React from 'react';

export function Card({ className = '', children, ...props }) {
  return (
    <div className={['vks-card', className].filter(Boolean).join(' ')} {...props}>
      {children}
    </div>
  );
}

export function CardHeader({ className = '', children, ...props }) {
  return <div className={['vks-card__header', className].filter(Boolean).join(' ')} {...props}>{children}</div>;
}

export function CardTitle({ className = '', children, ...props }) {
  return <h3 className={['vks-card__title', className].filter(Boolean).join(' ')} {...props}>{children}</h3>;
}

export function CardDescription({ className = '', children, ...props }) {
  return <p className={['vks-card__desc', className].filter(Boolean).join(' ')} {...props}>{children}</p>;
}

export function CardContent({ className = '', children, ...props }) {
  return <div className={['vks-card__content', className].filter(Boolean).join(' ')} {...props}>{children}</div>;
}

export function CardFooter({ className = '', children, ...props }) {
  return <div className={['vks-card__footer', className].filter(Boolean).join(' ')} {...props}>{children}</div>;
}
