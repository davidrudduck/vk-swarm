import { useState, useEffect, useCallback } from 'react';

export type SyncStatus = 'synced' | 'reconnecting' | 'disconnected';

let sharedLastUpdateAt = Date.now();

export function getSyncStatus(lastUpdateAt: number | null): SyncStatus {
  if (lastUpdateAt === null) return 'synced';
  const elapsed = Date.now() - lastUpdateAt;
  if (elapsed < 30_000) return 'synced';
  if (elapsed < 60_000) return 'reconnecting';
  return 'disconnected';
}

export function useSyncStatus() {
  const [syncStatus, setSyncStatus] = useState<SyncStatus>('synced');

  const markSynced = useCallback(() => {
    sharedLastUpdateAt = Date.now();
    setSyncStatus('synced');
  }, []);

  useEffect(() => {
    const tick = () => {
      const status = getSyncStatus(sharedLastUpdateAt);
      setSyncStatus(status);
    };

    const handleOnline = () => {
      sharedLastUpdateAt = Date.now();
      setSyncStatus('synced');
    };
    const handleOffline = () => setSyncStatus('disconnected');

    const interval = setInterval(tick, 10_000);
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      clearInterval(interval);
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return { syncStatus, markSynced };
}