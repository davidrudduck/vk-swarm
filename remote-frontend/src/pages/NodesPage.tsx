import { useQuery } from '@tanstack/react-query';
import { NodesView, type NodeRow } from '@/ui/panels';
import { nodesApi } from '@/lib/api/nodes';
import type { Node } from '@/types/nodes';
import { organizationsApi } from '@/lib/api/organizations';
import { NodeApiKeySection } from '@/components/swarm';
import { ErrorBoundary } from '@/components/ErrorBoundary';

/**
 * Wires `NodesView` to live data (SC8).
 *
 * Fetch chain: organizations -> first org -> nodes for that org. This
 * "pick the first org" selection is a placeholder until an org switcher
 * lands (same ledgered pattern as `BoardPage`, task 308).
 *
 * Preserves `NodeApiKeySection` (added to the old `pages/Nodes.tsx` in
 * #461, after this task's plan doc was authored -- the plan's "replace the
 * /nodes placeholder" wording predates that commit and is stale). Dropping
 * key management here would be a functional regression, so it is composed
 * alongside the new `NodesView` rather than orphaned (ledgered, task 309).
 *
 * `NodeApiKeySection` is wrapped in a local `ErrorBoundary` with a `null`
 * fallback: it renders on its own key-management data (`nodesApi.listApiKeys`)
 * independent from the `NodesView` node list above it, and should never take
 * down the page if that fetch fails or returns an unexpected shape.
 */
export function NodesPage() {
  const orgsQ = useQuery({ queryKey: ['orgs'], queryFn: organizationsApi.list });
  const orgId = orgsQ.data?.[0]?.id;

  const { data: nodes = [] } = useQuery({
    queryKey: ['nodes', orgId],
    queryFn: () => nodesApi.list(orgId!),
    enabled: !!orgId,
  });

  const rows = nodes.map(mapNodeToRow);
  return (
    <>
      {orgId && (
        <ErrorBoundary fallback={null}>
          <NodeApiKeySection organizationId={orgId} />
        </ErrorBoundary>
      )}
      <NodesView nodes={rows} />
    </>
  );
}

/**
 * Maps the real hive `Node` (`crates/remote/src/nodes/domain.rs`, see
 * `@/types/nodes`) to `NodeRow`. Field-gap (ledgered, task 309): the real
 * `Node` type has no `os_info`/`hostname` fields as some earlier plan prose
 * assumed -- OS comes from `capabilities.os`, and `meta` falls back to
 * `public_url` since there is no hostname field at all.
 */
function mapNodeToRow(n: Node): NodeRow {
  const os = n.capabilities?.os?.toLowerCase() ?? '';
  return {
    id: n.id,
    name: n.name,
    os: os.includes('mac') ? 'mac' : os.includes('win') ? 'windows' : 'linux',
    online: n.status === 'online',
    meta: n.public_url ?? '',
    rightCount: 0,
  };
}
