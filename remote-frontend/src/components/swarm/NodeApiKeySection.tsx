import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import { Loader2, Key, Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { TooltipProvider } from '@/components/ui/tooltip';
import { nodesApi } from '@/lib/api';
import type { NodeApiKey } from '@/types/nodes';

interface ApiKeyItemProps {
  apiKey: NodeApiKey;
  onRevoke: (keyId: string) => void;
}

function ApiKeyItem({ apiKey, onRevoke }: ApiKeyItemProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <div className="flex items-center justify-between p-3 border rounded-lg">
      <div className="flex items-center gap-3">
        <Key className="h-5 w-5 text-muted-foreground" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium text-sm">{apiKey.name}</span>
            <code className="text-xs text-muted-foreground">{apiKey.key_prefix}</code>
            <Badge variant={apiKey.node_id ? 'default' : 'secondary'}>
              {apiKey.node_id
                ? t('settings.swarm.apiKeys.bound', 'Bound')
                : t('settings.swarm.apiKeys.unbound', 'Unbound')}
            </Badge>
          </div>
          <div className="text-xs text-muted-foreground">
            {t('settings.swarm.apiKeys.created', 'Created {{when}}', {
              when: apiKey.created_at.slice(0, 10),
            })}
            {apiKey.last_used_at && (
              <>
                {' · '}
                {t('settings.swarm.apiKeys.lastUsed', 'Last used {{when}}', {
                  when: apiKey.last_used_at.slice(0, 10),
                })}
              </>
            )}
          </div>
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => onRevoke(apiKey.id)}
      >
        {t('settings.swarm.apiKeys.revoke', 'Revoke')}
      </Button>
    </div>
  );
}

export function NodeApiKeySection({
  organizationId,
}: {
  organizationId: string;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const [_showCreateDialog, setShowCreateDialog] = useState(false);

  const { data: apiKeys = [], isLoading } = useQuery({
    queryKey: ['nodeApiKeys', organizationId],
    queryFn: () => nodesApi.listApiKeys(organizationId),
    enabled: !!organizationId,
  });

  if (!organizationId) return null;

  return (
    <TooltipProvider>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Key className="h-5 w-5 text-muted-foreground" />
              <CardTitle className="text-lg">
                {t('settings.swarm.apiKeys.title', 'Node API Keys')}
              </CardTitle>
            </div>
            <Button
              onClick={() => setShowCreateDialog(true)}
              size="sm"
              className="gap-2"
            >
              <Plus className="h-4 w-4" />
              {t('settings.swarm.apiKeys.create', 'Generate API Key')}
            </Button>
          </div>
          <CardDescription>
            {t(
              'settings.swarm.apiKeys.description',
              'API keys allow nodes to authenticate with the hive server'
            )}
          </CardDescription>
        </CardHeader>

        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : apiKeys.length === 0 ? (
            <p className="text-center py-8 text-muted-foreground">
              {t(
                'settings.swarm.apiKeys.empty',
                'No API keys found. Create one to allow nodes to connect.'
              )}
            </p>
          ) : (
            <div className="space-y-3">
              {apiKeys.map((key) => (
                <ApiKeyItem
                  key={key.id}
                  apiKey={key}
                  onRevoke={() => {}}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </TooltipProvider>
  );
}
