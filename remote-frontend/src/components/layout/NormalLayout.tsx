import { Outlet } from 'react-router-dom';
import { Navbar } from '@/components/layout/Navbar';
import { BottomNav } from '@/components/layout/BottomNav';
import { useOnlineStatus } from '@/lib/offline';

export function NormalLayout() {
  const { isOnline } = useOnlineStatus();

  return (
    <>
      <Navbar />
      {!isOnline && (
        <div className="bg-amber-900/30 border-b border-amber-600/50 text-amber-200 text-sm text-center py-1.5">
          You're offline — changes will sync when reconnected
        </div>
      )}
      <div className="flex-1 min-h-0 overflow-hidden pb-14 sm:pb-0">
        <Outlet />
      </div>
      <BottomNav />
    </>
  );
}
