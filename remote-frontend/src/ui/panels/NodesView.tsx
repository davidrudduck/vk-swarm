import type { ReactElement } from 'react';
import { NodeCard, StatusBadge, type NodeOS } from '@/components/board';
import { Badge } from '@/components/core';

export interface NodeRow {
  id: string;
  name: string;
  os: NodeOS;
  online: boolean;
  meta: string;
  rightCount: number;
}

export interface NodesViewProps {
  nodes: NodeRow[];
}

/** Hive registry panel — grid of NodeCard entries. */
export function NodesView({ nodes }: NodesViewProps): ReactElement {
  const onlineCount = nodes.filter((n) => n.online).length;
  return (
    <div style={{ padding: 20, overflowY: 'auto', height: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <h2 style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)', fontWeight: 600, margin: 0 }}>
          Hive
        </h2>
        <StatusBadge status="done" label={`${onlineCount} nodes online`} />
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: 12, maxWidth: 1000 }}>
        {nodes.map((n) => (
          <NodeCard
            key={n.id}
            name={n.name}
            os={n.os}
            online={n.online}
            meta={n.meta}
            right={
              <Badge variant={n.online ? 'secondary' : 'outline'} dot={n.online}>
                {n.rightCount || 'offline'}
              </Badge>
            }
          />
        ))}
      </div>
    </div>
  );
}
