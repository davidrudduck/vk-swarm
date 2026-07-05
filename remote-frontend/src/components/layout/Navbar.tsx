import { Link, useLocation } from 'react-router-dom';
import { FolderOpen, ListTodo, LogOut } from 'lucide-react';
import { useState, useEffect } from 'react';
import { cn } from '@/lib/utils';
import { oauthApi } from '@/lib/api/oauth';
import { useSyncStatus } from '@/lib/electric/sync-status';
import { getQueueLength } from '@/lib/mutation-queue';

const NAV_ITEMS = [
  { label: 'Nodes', icon: FolderOpen, to: '/nodes' },
  { label: 'Tasks', icon: ListTodo, to: '/tasks' },
];

export function Navbar() {
  const location = useLocation();

  const { syncStatus } = useSyncStatus();
  const syncColor: Record<typeof syncStatus, string> = {
    synced: 'bg-green-500',
    reconnecting: 'bg-yellow-500',
    disconnected: 'bg-red-500',
  };

  const [queueLength, setQueueLength] = useState(0);

  useEffect(() => {
    const update = () => {
      getQueueLength().then(setQueueLength).catch(() => {});
    };
    update();
    const interval = setInterval(update, 5_000);
    return () => clearInterval(interval);
  }, []);

  const handleLogout = async () => {
    try {
      await oauthApi.logout();
      window.location.reload();
    } catch (err) {
      console.error('Error logging out:', err);
    }
  };

  return (
    <div className="border-b bg-background" data-testid="navbar">
      <div className="w-full px-3">
        <div className="flex items-center h-12 py-2">
          <div className="flex-1">
            <Link to="/nodes" className="text-foreground font-semibold flex items-center gap-2">
              VK Swarm
              <span
                className={`inline-block w-2 h-2 rounded-full ${syncColor[syncStatus]}`}
                title={`Sync: ${syncStatus}`}
                aria-label={`Sync status: ${syncStatus}`}
              />
              {queueLength > 0 && (
                <span className="inline-flex items-center justify-center px-1.5 py-0.5 text-xs bg-amber-500 text-black rounded-full font-bold">
                  {queueLength} pending
                </span>
              )}
            </Link>
          </div>
          <button
            onClick={handleLogout}
            className="inline-flex items-center gap-2 px-3 py-2 text-sm text-foreground hover:text-muted-foreground transition-colors"
            aria-label="Logout"
          >
            <LogOut className="h-4 w-4" />
            Logout
          </button>
        </div>

        <nav className="flex items-center gap-4 h-9 border-t text-sm">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname === item.to;
            return (
              <Link
                key={item.to}
                to={item.to}
                className={cn(
                  'mb-[-1px] py-2 flex items-center gap-2',
                  isActive
                    ? 'border-b-2 border-primary text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                <Icon className="h-4 w-4" />
                {item.label}
              </Link>
            );
          })}
        </nav>
      </div>
    </div>
  );
}
