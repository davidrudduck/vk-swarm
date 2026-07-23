import { Monitor } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { Node } from '@/types/nodes';

/**
 * The per-node element type the Nodes page (task 017) maps over.
 *
 * plan.md's contract nominally points at `useAvailableNodes`
 * (`ListProjectNodesResponse`), but that source is task-scoped and cannot feed
 * a global /nodes page. The only global node list is `nodesApi.list(orgId)` →
 * `Node[]` (the source NodeProjectsSection uses), whose `name` / `status` fields
 * match plan.md's stated `node.name` + `'online'|'offline'` access. So
 * NodeForCard === Node. (Limitation: org-scoped, no agent-count — see task file.)
 */
export type NodeForCard = Node;

interface NodeCardProps {
  node: NodeForCard;
  className?: string;
}

/**
 * Presentational card for one swarm node: a raised OS-glyph tile, the node name
 * in mono, and an online pulse dot (vks-pulse keyframe from task 003) or a dim
 * offline dot. Mirrors NodeProjectsSection's Monitor glyph + node.name + the
 * `node.status === 'online'` online test; restyled for the card surface.
 */
export function NodeCard({ node, className }: NodeCardProps) {
  const isOnline = node.status === 'online';

  return (
    <div
      className={cn(
        'flex items-center gap-3 rounded-md border border-border bg-[var(--surface-card)] p-3',
        className
      )}
    >
      {/* OS-glyph tile — 36x36 raised surface. (Future: OS-specific glyph from
          node.capabilities.os; sibling uses a single Monitor for all nodes.) */}
      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-[var(--surface-raised)]">
        <Monitor className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
      </div>

      {/* Online/offline dot — pulse only when online (SC15 application). */}
      {isOnline ? (
        <span
          className="h-2 w-2 shrink-0 rounded-full bg-[var(--status-done)] animate-[vks-pulse_2s_ease-in-out_infinite]"
          aria-hidden="true"
        />
      ) : (
        <span
          className="h-2 w-2 shrink-0 rounded-full bg-[var(--vks-text-dim)]"
          aria-hidden="true"
        />
      )}

      {/* Node name — mono, truncating. */}
      <span className="min-w-0 flex-1 truncate font-mono text-sm">
        {node.name}
      </span>

      {/* Right slot — offline badge, or a muted status label when online. No
          fabricated agent count (Node carries none). */}
      {isOnline ? (
        <span className="shrink-0 text-xs text-muted-foreground">
          {node.status}
        </span>
      ) : (
        <Badge variant="secondary" className="shrink-0">
          {node.status}
        </Badge>
      )}
    </div>
  );
}
