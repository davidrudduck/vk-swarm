import * as React from 'react';

export type NodeOS = 'mac' | 'linux' | 'windows';

export interface NodeCardProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Node hostname / display name. */
  name: string;
  /** Platform glyph. @default 'linux' */
  os?: NodeOS;
  /** Connected to the hive → emerald pulse; else dim. @default true */
  online?: boolean;
  /** Secondary line (URL, agent count, last seen). */
  meta?: React.ReactNode;
  /** Trailing slot (badge, button). */
  right?: React.ReactNode;
}

/** A swarm node in the hive registry list. */
export function NodeCard(props: NodeCardProps): React.ReactElement;
