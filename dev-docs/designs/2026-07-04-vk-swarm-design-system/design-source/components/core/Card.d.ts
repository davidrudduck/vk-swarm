import * as React from 'react';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {}

/** Surface container on `--surface-card` with hairline border. */
export function Card(props: CardProps): React.ReactElement;
export function CardHeader(props: CardProps): React.ReactElement;
export function CardTitle(props: React.HTMLAttributes<HTMLHeadingElement>): React.ReactElement;
export function CardDescription(props: React.HTMLAttributes<HTMLParagraphElement>): React.ReactElement;
export function CardContent(props: CardProps): React.ReactElement;
export function CardFooter(props: CardProps): React.ReactElement;
