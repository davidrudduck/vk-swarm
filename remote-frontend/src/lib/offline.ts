import { useState, useEffect, useCallback } from 'react';

interface OnlineStatus {
  isOnline: boolean;
  wasOffline: boolean;
  lastOnlineAt: Date | null;
}

export function useOnlineStatus(): OnlineStatus {
  const [isOnline, setIsOnline] = useState(navigator.onLine);
  const [wasOffline, setWasOffline] = useState(false);
  const [lastOnlineAt, setLastOnlineAt] = useState<Date | null>(null);

  const handleOnline = useCallback(() => {
    setIsOnline(true);
    setLastOnlineAt(new Date());
  }, []);

  const handleOffline = useCallback(() => {
    setIsOnline(false);
    setWasOffline(true);
  }, []);

  useEffect(() => {
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [handleOnline, handleOffline]);

  return { isOnline, wasOffline, lastOnlineAt };
}
