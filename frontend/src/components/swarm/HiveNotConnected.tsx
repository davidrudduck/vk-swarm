import { useTranslation } from 'react-i18next';
import { Alert, AlertDescription } from '@/components/ui/alert';

/**
 * Shown in place of swarm-facing sections when this node is not connected to
 * a hive (server returned HTTP 503 / ApiError::HiveNotConfigured). Swarm
 * management lives on the hive, so there is nothing to load locally.
 */
export function HiveNotConnected() {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <div className="px-4 pb-4 sm:px-6">
      <Alert>
        <AlertDescription>
          {t(
            'settings.swarm.hiveNotConnected',
            'This node is not connected to a hive. Swarm management lives on the hive server — connect a hive to manage swarm projects, labels, and templates.'
          )}
        </AlertDescription>
      </Alert>
    </div>
  );
}
