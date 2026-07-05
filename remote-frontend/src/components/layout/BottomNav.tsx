import { useLocation, useNavigate } from 'react-router-dom';
import { FolderOpen, ListTodo } from 'lucide-react';
import { cn } from '@/lib/utils';

interface NavItemProps {
  icon: React.ElementType;
  label: string;
  isActive: boolean;
  onClick: () => void;
}

function NavItem({ icon: Icon, label, isActive, onClick }: NavItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex flex-col items-center justify-center gap-0.5 h-12 min-w-[48px] px-3 rounded-lg transition-colors',
        isActive
          ? 'text-foreground font-medium'
          : 'text-muted-foreground hover:text-foreground'
      )}
      aria-label={label}
      aria-current={isActive ? 'page' : undefined}
    >
      <Icon className="h-5 w-5" />
      <span className="text-[10px] font-medium">{label}</span>
    </button>
  );
}

export function BottomNav() {
  const location = useLocation();
  const navigate = useNavigate();

  const isNodesActive = location.pathname === '/nodes';
  const isTasksActive = location.pathname === '/tasks';

  return (
    <nav
      role="navigation"
      aria-label="Main navigation"
      className="fixed bottom-0 left-0 right-0 z-50 h-14 bg-background border-t border-border sm:hidden safe-area-bottom"
    >
      <div className="flex items-center justify-around h-full max-w-md mx-auto px-2">
        <NavItem
          icon={FolderOpen}
          label="Nodes"
          isActive={isNodesActive}
          onClick={() => navigate('/nodes')}
        />
        <NavItem
          icon={ListTodo}
          label="Tasks"
          isActive={isTasksActive}
          onClick={() => navigate('/tasks')}
        />
      </div>
    </nav>
  );
}
