import type { TaskStatus } from '@/components/board';

export interface ColumnDef {
  key: TaskStatus;
  label: string;
  color: string;
}

export const COLUMNS: ColumnDef[] = [
  { key: 'todo', label: 'To Do', color: 'var(--status-todo)' },
  { key: 'inprogress', label: 'In Progress', color: 'var(--status-inprogress)' },
  { key: 'inreview', label: 'In Review', color: 'var(--status-inreview)' },
  { key: 'done', label: 'Done', color: 'var(--status-done)' },
  { key: 'cancelled', label: 'Cancelled', color: 'var(--status-cancelled)' },
];
