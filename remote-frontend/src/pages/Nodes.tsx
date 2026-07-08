import { useQuery } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { NodeCard } from '@/components/swarm/NodeCard';
import { NodeApiKeySection } from '@/components/swarm';
import { nodesApi } from '@/lib/api';
import { useOrganizations } from '@/hooks/useOrganizations';

export function Nodes() {
  const { data: organizations = [], isLoading: orgsLoading } =
    useOrganizations();
  // For the hive, use the first organization (no is_personal filter)
  const orgId = organizations[0]?.id;

  const {
    data: nodes = [],
    isLoading: nodesLoading,
    isError,
  } = useQuery({
    queryKey: ['nodes', orgId],
    queryFn: () => nodesApi.list(orgId!),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  const isLoading = orgsLoading || (!!orgId && nodesLoading);

  return (
    <div className="space-y-6 p-8 pb-16 md:pb-8 h-full overflow-auto">
      <h2 className="font-serif text-2xl font-semibold">Nodes</h2>

      {orgId && <NodeApiKeySection organizationId={orgId} />}

      {isLoading ? (
        <div className="flex items-center justify-center py-8" role="status">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          <span className="sr-only">Loading nodes...</span>
        </div>
      ) : !orgId ? (
        <p className="text-muted-foreground">
          Nodes are a swarm feature. Connect a hive server to get started.
        </p>
      ) : isError ? (
        <p className="text-muted-foreground">Failed to load nodes.</p>
      ) : nodes.length === 0 ? (
        <p className="text-muted-foreground">No nodes connected yet.</p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3 max-w-[1000px]">
          {nodes.map((node) => (
            <NodeCard key={node.id} node={node} />
          ))}
        </div>
      )}
    </div>
  );
}
