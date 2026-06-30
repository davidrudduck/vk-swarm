/**
 * Utility functions for calculating and displaying "days in column" badge
 * for task cards. Shows how long a task has been in its current status.
 */

/**
 * Calculate the number of days since the activity date.
 * Returns 0 if activityAt is null or within the last 24 hours.
 */
export function getDaysInColumn(
  activityAt: Date | string | null | undefined
): number {
  if (!activityAt) return 0;

  const activityDate =
    typeof activityAt === 'string' ? new Date(activityAt) : activityAt;

  // Handle invalid dates
  if (isNaN(activityDate.getTime())) return 0;

  const now = new Date();
  const diffMs = now.getTime() - activityDate.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  return Math.max(0, diffDays);
}

/**
 * Format days into a display string.
 * Returns null if days is 0 (< 1 day old).
 * Returns the literal "{n}d" for any day count >= 1 (no upper cap).
 */
export function formatDaysInColumn(days: number): string | null {
  if (days < 1) return null;
  return `${days}d`;
}
