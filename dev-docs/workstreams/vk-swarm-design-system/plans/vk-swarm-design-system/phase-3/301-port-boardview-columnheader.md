---
id: "301"
phase: 3
title: Port BoardView + ColumnHeader into remote-frontend
status: passed
depends_on: ["205","206"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/ui/board/BoardView.tsx
  - remote-frontend/src/ui/board/columns.ts
  - remote-frontend/src/ui/board/index.ts
  - remote-frontend/src/ui/board/boardview.test.tsx
irreversible: false
scope_test: "remote-frontend/src/ui/board/boardview.test.tsx"
allowed_change: create
covers_criteria: [SC7]
---

## Sibling alignment

Read `design-source/ui_kits/vk-swarm-app/board.jsx` (85 lines). It defines `COLUMNS` (5 fixed columns: todo/inprogress/inreview/done/cancelled with label+color), `ColumnHeader` (sticky header with status dot, label, count badge, add button), and `BoardView` (CSS grid `gridAutoFlow: 'column' gridAutoColumns: 'minmax(264px, 1fr)'`, per-column flex col, TaskCard list with `onClick=onOpen`, selected ring, empty-state `vks-ansi-dither vks-scanlines` with `░▒ no tasks ▒░`). The TS port replaces the `window.VKSwarmDesignSystem_067861.TaskCard` lookup with a direct `import { TaskCard } from '@/components/board'` (task 205). The `columns` prop shape: `Record<TaskStatus, TaskRow[]>` where `TaskRow` is `{ id: string; title: string; description?: string; node?: string; labels?: string[]; attempt?: AttemptState; days?: number }`. Keep `COLUMNS` as a separate `columns.ts` module so task 308 (data wiring) can pass a `columns` prop computed from live data instead of `SEED`. Record any divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/ui/board/boardview.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { BoardView, COLUMNS } from './index';

const columns = {
  todo: [{ id: 't1', title: 'First', description: 'd', node: 'n1', labels: ['a'], days: 1 }],
  inprogress: [{ id: 't2', title: 'Second', node: 'n2', labels: [], attempt: 'running' as const, days: 2 }],
  inreview: [],
  done: [],
  cancelled: [],
};

describe('BoardView (SC7)', () => {
  it('renders one column per COLUMNS entry with the label', () => {
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} />);
    for (const col of COLUMNS) expect(screen.getByText(col.label)).toBeTruthy();
  });

  it('renders a TaskCard per row in each column', () => {
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} />);
    expect(screen.getByText('First')).toBeTruthy();
    expect(screen.getByText('Second')).toBeTruthy();
  });

  it('renders the empty-state texture for empty columns', () => {
    const { container } = render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} />);
    expect(container.querySelector('.vks-ansi-dither')).toBeTruthy();
    expect(container.querySelector('.vks-scanlines')).toBeTruthy();
    expect(screen.getByText(/no tasks/)).toBeTruthy();
  });

  it('calls onOpen(task, statusKey) when a TaskCard is clicked', () => {
    const onOpen = vi.fn();
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={onOpen} />);
    fireEvent.click(screen.getByText('First'));
    expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ id: 't1' }), 'todo');
  });

  it('applies the selected ring when selectedId matches a task id', () => {
    const { container } = render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} selectedId="t1" />);
    const card = screen.getByText('First').closest('.vks-task') as HTMLElement;
    expect(card.style.boxShadow).toContain('var(--primary)');
  });
});
```

## Change

### File: `remote-frontend/src/ui/board/columns.ts` (CREATE)
```ts
import type { TaskStatus } from '@/components/board';

export interface ColumnDef { key: TaskStatus; label: string; color: string; }

export const COLUMNS: ColumnDef[] = [
  { key: 'todo', label: 'To Do', color: 'var(--status-todo)' },
  { key: 'inprogress', label: 'In Progress', color: 'var(--status-inprogress)' },
  { key: 'inreview', label: 'In Review', color: 'var(--status-inreview)' },
  { key: 'done', label: 'Done', color: 'var(--status-done)' },
  { key: 'cancelled', label: 'Cancelled', color: 'var(--status-cancelled)' },
];
```

### File: `remote-frontend/src/ui/board/BoardView.tsx` (CREATE)
TypeScript port of `board.jsx`. `TaskRow = { id: string; title: string; description?: string; node?: string; labels?: string[]; attempt?: import('@/components/board').AttemptState; days?: number }`. `BoardViewProps { columns: Record<TaskStatus, TaskRow[]>; onAdd: (status: TaskStatus) => void; onOpen: (task: TaskRow, status: TaskStatus) => void; selectedId?: string }`. `ColumnHeader({ col, count, onAdd })` renders the sticky header (status dot, label, count badge, add button). `BoardView` renders the CSS grid with one column per `COLUMNS` entry, mapping `columns[col.key]` to `<TaskCard>` with `onClick={() => onOpen(t, col.key)}` and `style={selectedId === t.id ? { boxShadow: '0 0 0 2px var(--primary)', borderColor: 'var(--primary)' } : undefined}`. Empty column renders `<div className="vks-ansi-dither vks-scanlines">░▒ no tasks ▒░</div>`.

### File: `remote-frontend/src/ui/board/index.ts` (CREATE)
`export { BoardView } from './BoardView'; export { COLUMNS } from './columns'; export type { TaskRow } from './BoardView';`

### File: `remote-frontend/src/ui/board/boardview.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 4 files as specified.
- Import `TaskCard` + `TaskStatus` + `AttemptState` from `@/components/board` (task 205).
- Preserve the inline-style approach of the source JSX (the design source uses inline styles for the grid layout; do NOT rewrite as Tailwind classes — the vks-* CSS classes are for the primitives, the layout is inline).
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- `TaskCard` or `TaskStatus` not exported from `@/components/board` (task 205 drift → STOP).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/ui/board/boardview.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 301` exits 0.