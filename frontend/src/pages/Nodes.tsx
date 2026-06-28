import { useQuery } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { NodeCard } from '@/components/swarm/NodeCard';
import { nodesApi } from '@/lib/api';
import { useUserOrganizations } from '@/hooks';

export function Nodes() {
  const { data: orgData } = useUserOrganizations();
  const organizations = orgData?.organizations ?? [];
  // Same default as useOrganizationSelection: first non-personal, else first.
  const orgId =
    (organizations.find((o) => !o.is_personal) ?? organizations[0])?.id;

  const {
    data: nodes = [],
    isLoading,
  } = useQuery({
    queryKey: ['nodes', orgId],
    queryFn: () => nodesApi.list(orgId!),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  return (
    <div className="space-y-6 p-8 pb-16 md:pb-8 h-full overflow-auto">
      <h2 className="font-serif text-2xl font-semibold">Nodes</h2>

      {!orgId || isLoading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
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
