import { Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useTheme } from '@/components/ThemeProvider';
import { useUserSystem } from '@/components/ConfigProvider';
import { ThemeMode } from 'shared/types';

interface ThemeToggleProps {
  className?: string;
}

/**
 * Ghost icon button toggling between DARK and LIGHT themes.
 *
 * Persistence: `setTheme` only updates React state, so we also call
 * `updateAndSaveConfig({ theme })` (mirroring GeneralSettings) to persist the
 * choice to config (SC20). Toggles binary DARK<->LIGHT only (never SYSTEM).
 */
export function ThemeToggle({ className }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();
  const { updateAndSaveConfig } = useUserSystem();

  const isDark = theme === ThemeMode.DARK;

  const handleToggle = async () => {
    const next = isDark ? ThemeMode.LIGHT : ThemeMode.DARK;
    try {
      await updateAndSaveConfig({ theme: next });
      setTheme(next);
    } catch (error) {
      console.error('Failed to persist theme preference', error);
    }
  };

  return (
    <Button
      variant="ghost"
      size="icon"
      className={cn('h-9 w-9', className)}
      onClick={handleToggle}
      // TODO(i18n): vk-swarm-node-ui-localize
      aria-label="Toggle theme"
    >
      {isDark ? (
        <Moon className="h-4 w-4" />
      ) : (
        <Sun className="h-4 w-4" />
      )}
    </Button>
  );
}

export default ThemeToggle;
