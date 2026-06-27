import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Cloud, Loader2 } from 'lucide-react';
import { databaseApi } from '@/lib/api/database';

function formatDate(date: Date | string | null): string {
  if (!date) return 'Never';
  const d = new Date(date);
  return d.toLocaleString();
}

function StatRow({
  label,
  value,
}: {
  label: string;
  value: string | number | bigint | boolean | null | undefined;
}) {
  const displayValue =
    typeof value === 'boolean' ? (value ? 'Connected' : 'Disconnected') : value ?? 'Not configured';
  return (
    <div className="flex justify-between py-2 border-b last:border-b-0">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono">{String(displayValue)}</span>
    </div>
  );
}

export function HiveSyncStatusCard() {
  const { t } = useTranslation('settings');

  const {
    data: syncStatus,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['hiveSyncStatus'],
    queryFn: databaseApi.getSyncStatus,
  });

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between">
        <div className="space-y-1">
          <CardTitle className="flex items-center gap-2">
            <Cloud className="h-5 w-5" />
            {t('settings.hiveSync.title', { defaultValue: 'Hive Sync Status' })}
          </CardTitle>
          <CardDescription>
            {t('settings.hiveSync.description', {
              defaultValue: 'View your Hive node configuration and sync status',
            })}
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="ml-2 text-muted-foreground">
              {t('settings.hiveSync.loading', { defaultValue: 'Loading sync status...' })}
            </span>
          </div>
        ) : error ? (
          <div className="text-center py-8 text-destructive">
            {t('settings.hiveSync.loadError', { defaultValue: 'Failed to load sync status' })}
          </div>
        ) : syncStatus ? (
          <div className="space-y-4">
            {/* Connection Status Section */}
            <div>
              <h3 className="text-sm font-semibold mb-2">
                {t('settings.hiveSync.section.connection', { defaultValue: 'Connection' })}
              </h3>
              <div className="border rounded-lg p-3 space-y-2">
                <StatRow label="Status" value={syncStatus.is_connected} />
                <StatRow label="Node ID" value={syncStatus.node_id || 'Not connected'} />
                <StatRow label="Node Name" value={syncStatus.node_name} />
                <StatRow label="Hive URL" value={syncStatus.hive_url} />
              </div>
            </div>

            {/* Sync Status Section */}
            <div>
              <h3 className="text-sm font-semibold mb-2">
                {t('settings.hiveSync.section.syncStatus', { defaultValue: 'Sync Status' })}
              </h3>
              <div className="border rounded-lg p-3 space-y-2">
                <StatRow label="Last Synced" value={formatDate(syncStatus.last_synced_at)} />
                <StatRow label="Unsynced Tasks" value={syncStatus.unsynced_tasks} />
                <StatRow label="Unsynced Attempts" value={syncStatus.unsynced_attempts} />
                <StatRow label="Unsynced Executions" value={syncStatus.unsynced_executions} />
                <StatRow label="Unsynced Logs" value={syncStatus.unsynced_logs} />
              </div>
            </div>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
