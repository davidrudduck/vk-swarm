import { cn } from '@/lib/utils';
import { getDaysInColumn, formatDaysInColumn } from '@/utils/daysInColumn';

interface DaysInColumnBadgeProps {
  /** The activity timestamp for the task (when it entered current status) */
  activityAt: Date | string | null | undefined;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Badge component showing how many days a task has been in its current column.
 * Returns null if less than 1 day old. Renders a flat, neutral `secondary` badge
 * regardless of age (no age-graduated colours).
 */
export function DaysInColumnBadge({
  activityAt,
  className,
}: DaysInColumnBadgeProps) {
  const days = getDaysInColumn(activityAt);
  const formatted = formatDaysInColumn(days);

  // Don't render if less than 1 day
  if (!formatted) return null;

  return (
    <span
      className={cn(
        'inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium',
        'bg-secondary text-secondary-foreground',
        className
      )}
      title={`${days} day${days === 1 ? '' : 's'} in this column`}
    >
      {formatted}
    </span>
  );
}
