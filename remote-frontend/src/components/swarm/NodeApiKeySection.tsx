import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Loader2, Key, Plus, Copy, Check, Eye, EyeOff, Trash2, Unlock, AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { nodesApi } from '@/lib/api';
import type { NodeApiKey } from '@/types/nodes';

function parseErrorMessage(err: unknown): string {
  let raw: string;
  if (err instanceof Error) {
    raw = err.message;
  } else if (typeof err === 'string') {
    raw = err || 'Failed';
  } else if (err == null) {
    return 'Failed';
  } else if (typeof err === 'symbol') {
    return 'Failed';
  } else {
    try {
      raw = JSON.stringify(err) ?? 'Failed';
    } catch {
      return 'Failed';
    }
  }
  if (!raw) return 'Failed';
  try {
    const parsed = JSON.parse(raw);
    if (parsed !== null && typeof parsed === 'object') {
      if (typeof parsed.message === 'string' && parsed.message) return parsed.message;
      if (typeof parsed.error === 'string' && parsed.error) return parsed.error;
      return 'Failed';
    }
    return raw || 'Failed';
  } catch {
    return raw || 'Failed';
  }
}

interface ApiKeyItemProps {
  apiKey: NodeApiKey;
  onRevoke: (keyId: string) => void;
  onUnblock: (keyId: string) => void;
  isPending: boolean;
}

function ApiKeyItem({ apiKey, onRevoke, onUnblock, isPending }: ApiKeyItemProps) {
  const { t } = useTranslation(['settings', 'common']);
  const isBlocked = apiKey.blocked_at !== null;
  const isRevoked = apiKey.revoked_at !== null;

  return (
    <div className="flex items-center justify-between p-3 border rounded-lg">
      <div className="flex items-center gap-3">
        <Key className="h-5 w-5 text-muted-foreground" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium text-sm truncate">{apiKey.name}</span>
            <code className="text-xs text-muted-foreground">{apiKey.key_prefix}</code>
            {isRevoked ? (
              <Badge variant="secondary">
                {t('settings.swarm.apiKeys.revoked', 'Revoked')}
              </Badge>
            ) : isBlocked && apiKey.blocked_reason ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span>
                    <Badge variant="destructive">
                      <AlertTriangle className="h-3 w-3 mr-1" />
                      {t('settings.swarm.apiKeys.blocked', 'Blocked')}
                    </Badge>
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{apiKey.blocked_reason}</p>
                </TooltipContent>
              </Tooltip>
            ) : isBlocked ? (
              <Badge variant="destructive">
                <AlertTriangle className="h-3 w-3 mr-1" />
                {t('settings.swarm.apiKeys.blocked', 'Blocked')}
              </Badge>
            ) : (
              <Badge variant={apiKey.node_id ? 'default' : 'secondary'}>
                {apiKey.node_id
                  ? t('settings.swarm.apiKeys.bound', 'Bound')
                  : t('settings.swarm.apiKeys.unbound', 'Unbound')}
              </Badge>
            )}
          </div>
          {!isRevoked && isBlocked && apiKey.blocked_reason && (
            <div className="text-xs text-destructive mt-1">
              {apiKey.blocked_reason}
            </div>
          )}
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
      {isRevoked ? null : isBlocked ? (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onUnblock(apiKey.id)}
          disabled={isPending}
        >
          <Unlock className="h-4 w-4 mr-1" />
          {t('settings.swarm.apiKeys.unblock', 'Unblock')}
        </Button>
      ) : (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onRevoke(apiKey.id)}
          disabled={isPending}
        >
          <Trash2 className="h-4 w-4 mr-1" />
          {t('settings.swarm.apiKeys.revoke', 'Revoke')}
        </Button>
      )}
    </div>
  );
}

export function NodeApiKeySection({
  organizationId,
}: {
  organizationId: string;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [showSecret, setShowSecret] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingKeyCount, setPendingKeyCount] = useState<Map<string, number>>(new Map());
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout>>();
  const createAttemptRef = useRef(0);
  const orgIdRef = useRef(organizationId);
  useEffect(() => { orgIdRef.current = organizationId; }, [organizationId]);

  const isMountedRef = useRef(true);
  useEffect(() => {
    isMountedRef.current = true;
    return () => { isMountedRef.current = false; };
  }, []);
  useEffect(() => () => clearTimeout(copyTimeoutRef.current), []);
  // Reset all create/pending state when organizationId changes.
  // We intentionally run only on org change, not on every mutation/dialog state change.
  useEffect(() => {
    setError(null);
    setPendingKeyCount(new Map());
    if (showCreateDialog) {
      createAttemptRef.current += 1;
      createMutation.reset();
      setShowCreateDialog(false);
      setNewKeyName('');
      setCreatedSecret(null);
      setShowSecret(false);
      setCopied(false);
      clearTimeout(copyTimeoutRef.current);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [organizationId]);

  const { data: apiKeys = [], isLoading, isError: isListError } = useQuery({
    queryKey: ['nodeApiKeys', organizationId],
    queryFn: () => nodesApi.listApiKeys(organizationId),
    enabled: !!organizationId,
    staleTime: 30_000,
  });

  const queryClient = useQueryClient();
  const createMutation = useMutation({
    mutationFn: ({ name, orgId }: { name: string; orgId: string }) =>
      nodesApi.createApiKey({ organization_id: orgId, name }),
    onMutate: () => createAttemptRef.current,
    onSuccess: (response, { orgId }, attemptId) => {
      queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', orgId] });
      if (attemptId !== createAttemptRef.current) return;
      if (orgId !== orgIdRef.current) return;
      if (!isMountedRef.current) return;
      setError(null);
      setCreatedSecret(response.secret);
      setNewKeyName('');
    },
    onError: (err, { orgId }, attemptId) => {
      if (attemptId !== createAttemptRef.current) return;
      if (orgId !== orgIdRef.current) return;
      if (!isMountedRef.current) return;
      setError(parseErrorMessage(err));
    },
  });

  const revokeMutation = useMutation({
    mutationFn: ({ keyId }: { keyId: string; orgId: string }) => nodesApi.revokeApiKey(keyId),
    onSuccess: (_data, { orgId }) => {
      if (isMountedRef.current && orgId === orgIdRef.current) setError(null);
      queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', orgId] });
    },
    onError: (err, { orgId }) => {
      if (!isMountedRef.current) return;
      if (orgId !== orgIdRef.current) return;
      setError(parseErrorMessage(err));
    },
  });
  const unblockMutation = useMutation({
    mutationFn: ({ keyId }: { keyId: string; orgId: string }) => nodesApi.unblockApiKey(keyId),
    onSuccess: (_data, { orgId }) => {
      if (isMountedRef.current && orgId === orgIdRef.current) setError(null);
      queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', orgId] });
    },
    onError: (err, { orgId }) => {
      if (!isMountedRef.current) return;
      if (orgId !== orgIdRef.current) return;
      setError(parseErrorMessage(err));
    },
  });

  const handleRevoke = (keyId: string) => {
    if (!confirm(t('settings.swarm.apiKeys.revokeConfirm', 'Are you sure you want to revoke this API key? Nodes using it will no longer be able to connect.'))) return;
    setError(null);
    if (isMountedRef.current) setPendingKeyCount(prev => new Map(prev).set(keyId, (prev.get(keyId) ?? 0) + 1));
    revokeMutation.mutate({ keyId, orgId: orgIdRef.current }, {
      onSettled: (_data, _err, vars) => {
        if (vars.orgId !== orgIdRef.current) return;
        if (isMountedRef.current) setPendingKeyCount(prev => { const next = new Map(prev); const count = (next.get(keyId) ?? 1) - 1; if (count <= 0) next.delete(keyId); else next.set(keyId, count); return next; });
      },
    });
  };
  const handleUnblock = (keyId: string) => {
    if (!confirm(t('settings.swarm.apiKeys.unblockConfirm', 'Are you sure you want to unblock this API key? The node will be able to connect again.'))) return;
    setError(null);
    if (isMountedRef.current) setPendingKeyCount(prev => new Map(prev).set(keyId, (prev.get(keyId) ?? 0) + 1));
    unblockMutation.mutate({ keyId, orgId: orgIdRef.current }, {
      onSettled: (_data, _err, vars) => {
        if (vars.orgId !== orgIdRef.current) return;
        if (isMountedRef.current) setPendingKeyCount(prev => { const next = new Map(prev); const count = (next.get(keyId) ?? 1) - 1; if (count <= 0) next.delete(keyId); else next.set(keyId, count); return next; });
      },
    });
  };

  const handleCreateSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (createMutation.isPending) return;
    const trimmed = newKeyName.trim();
    if (!trimmed) return;
    setError(null);
    createMutation.mutate({ name: trimmed, orgId: orgIdRef.current });
  };

  const closeDialog = () => {
    createAttemptRef.current += 1;
    createMutation.reset();
    setShowCreateDialog(false);
    setNewKeyName('');
    setCreatedSecret(null);
    setShowSecret(false);
    setCopied(false);
    clearTimeout(copyTimeoutRef.current);
  };

  const handleCopySecret = async () => {
    if (!createdSecret) return;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(createdSecret);
      } else {
        const ta = document.createElement('textarea');
        ta.value = createdSecret;
        document.body.appendChild(ta);
        try {
          ta.focus();
          ta.select();
          if (!document.execCommand('copy')) {
            throw new Error('Fallback copy failed');
          }
        } finally {
          document.body.removeChild(ta);
        }
      }
      setCopied(true);
      clearTimeout(copyTimeoutRef.current);
      copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000);
    } catch {
      console.error('Failed to copy to clipboard');
    }
  };

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

        {error && (
          <div className="px-6 pb-4">
            <Alert variant="destructive">
              <AlertDescription>
                {t('settings.swarm.apiKeys.error', 'Failed: {{message}}', { message: error })}
              </AlertDescription>
            </Alert>
          </div>
        )}

        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8" role="status">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              <span className="sr-only">{t('settings.swarm.apiKeys.loading', 'Loading API keys...')}</span>
            </div>
          ) : isListError ? (
            <Alert variant="destructive">
              <AlertDescription>
                {t('settings.swarm.apiKeys.loadError', 'Failed to load API keys.')}
              </AlertDescription>
            </Alert>
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
                  onRevoke={handleRevoke}
                  onUnblock={handleUnblock}
                  isPending={(pendingKeyCount.get(key.id) ?? 0) > 0}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog
        open={showCreateDialog}
        onOpenChange={(open) => {
          if (!open && !createdSecret && !createMutation.isPending) closeDialog();
        }}
        uncloseable={!!createdSecret || createMutation.isPending}
      >
        <DialogContent>
          {!createdSecret ? (
            <form onSubmit={handleCreateSubmit}>
              <DialogHeader>
                <DialogTitle>
                  {t('settings.swarm.apiKeys.createTitle', 'Generate API Key')}
                </DialogTitle>
                <DialogDescription>
                  {t('settings.swarm.apiKeys.createDescription', 'Give your API key a name to identify it later.')}
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="api-key-name">
                    {t('settings.swarm.apiKeys.nameLabel', 'Key Name')}
                  </Label>
                  <Input
                    id="api-key-name"
                    value={newKeyName}
                    onChange={(e) => setNewKeyName(e.target.value)}
                    placeholder={t('settings.swarm.apiKeys.namePlaceholder', 'e.g. Production Node')}
                    maxLength={100}
                    autoFocus
                  />
                </div>
              </div>
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  disabled={createMutation.isPending}
                  onClick={closeDialog}
                >
                  {t('settings.swarm.apiKeys.cancel', 'Cancel')}
                </Button>
                <Button
                  type="submit"
                  disabled={!newKeyName.trim() || createMutation.isPending}
                >
                  {t('settings.swarm.apiKeys.createAction', 'Create')}
                </Button>
              </DialogFooter>
            </form>
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>
                  {t('settings.swarm.apiKeys.secretTitle', 'API Key Created')}
                </DialogTitle>
                <DialogDescription>
                  {t('settings.swarm.apiKeys.secretDescription', 'Copy this secret now. It will not be shown again.')}
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <code
                  data-secret-wrapper
                  data-hidden={!showSecret}
                  className="block p-3 rounded bg-muted text-sm break-all"
                  aria-label={showSecret ? t('settings.swarm.apiKeys.secretVisible', 'API key secret') : t('settings.swarm.apiKeys.secretHidden', 'API key secret (hidden)')}
                >
                  {showSecret ? createdSecret : '••••••••••••••••••••'}
                </code>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    aria-pressed={showSecret}
                    onClick={() => setShowSecret(!showSecret)}
                  >
                    {showSecret ? (
                      <>
                        <EyeOff className="h-4 w-4 mr-2" />
                        {t('settings.swarm.apiKeys.hideSecret', 'Hide')}
                      </>
                    ) : (
                      <>
                        <Eye className="h-4 w-4 mr-2" />
                        {t('settings.swarm.apiKeys.showSecret', 'Reveal')}
                      </>
                    )}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleCopySecret}
                    aria-live="polite"
                  >
                    {copied ? (
                      <>
                        <Check className="h-4 w-4 mr-2" />
                        {t('settings.swarm.apiKeys.copied', 'Copied!')}
                      </>
                    ) : (
                      <>
                        <Copy className="h-4 w-4 mr-2" />
                        {t('settings.swarm.apiKeys.copyToClipboard', 'Copy')}
                      </>
                    )}
                  </Button>
                </div>
              </div>
              <DialogFooter>
                <Button onClick={closeDialog}>
                  {t('settings.swarm.apiKeys.done', 'Done')}
                </Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </TooltipProvider>
  );
}
