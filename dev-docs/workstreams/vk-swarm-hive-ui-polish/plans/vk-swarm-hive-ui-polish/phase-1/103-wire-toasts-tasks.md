---
id: "103"
phase: 1
title: Wire toasts + confirmation dialogs + loading states into Tasks.tsx
status: ready
depends_on: ["100"]
parallel: false
conflicts_with: [205]
files:
  - remote-frontend/src/pages/Tasks.tsx
  - remote-frontend/src/pages/Tasks.test.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Tasks"
allowed_change: edit
covers_criteria: [SC3, SC4, SC5]
---

## Failing test (write first)

The existing test at `remote-frontend/src/pages/Tasks.test.tsx` is the gate. Edit it to assert the new behavior.

Replace `remote-frontend/src/pages/Tasks.test.tsx` with:

```tsx
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TasksBoard } from './Tasks';
import { tasksApi } from '@/lib/api/tasks';

vi.mock('@/lib/api/tasks', () => ({
  tasksApi: {
    setExecutingNode: vi.fn(),
    delete: vi.fn(),
  },
}));

vi.mock('@/lib/electric', () => ({
  createTaskAssignmentsCollection: vi.fn(() => vi.fn()),
  createTaskOutputLogsCollection: vi.fn(() => vi.fn()),
  createTaskProgressEventsCollection: vi.fn(() => vi.fn()),
  createNodesCollection: vi.fn(() => vi.fn()),
  createProjectsCollection: vi.fn(() => vi.fn()),
}));

vi.mock('@tanstack/react-db', () => ({
  useLiveQuery: vi.fn(() => ({ data: [] })),
}));

import { useLiveQuery } from '@tanstack/react-db';
const mockedUseLiveQuery = useLiveQuery as ReturnType<typeof vi.fn>;

describe('Tasks.tsx error resilience (SC3, SC4, SC5)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedUseLiveQuery.mockReturnValue({ data: [] });
  });

  it('shows loading state on assign button during mutation', async () => {
    (tasksApi.setExecutingNode as ReturnType<typeof vi.fn>).mockReturnValue(
      new Promise(() => {}),
    );

    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
        { id: 'p1', name: 'proj-1' },
      ],
    });

    render(<TasksBoard />);

    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'n1' } });

    const assignBtn = screen.getByLabelText('Assign');
    fireEvent.click(assignBtn);

    await waitFor(() => {
      expect(assignBtn).toBeDisabled();
    });
  });

  it('shows delete confirmation dialog before dispatching DELETE', async () => {
    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
      ],
    });

    render(<TasksBoard />);

    const deleteBtn = screen.getByLabelText('Delete');
    fireEvent.click(deleteBtn);

    await waitFor(() => {
      expect(screen.getByText('Are you sure?')).toBeDefined();
    });
  });

  it('dispatches DELETE when confirm is clicked and shows success toast', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ ok: true });

    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
      ],
    });

    render(<TasksBoard />);

    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => {
      expect(screen.getByText('Are you sure?')).toBeDefined();
    });

    fireEvent.click(screen.getByText('Delete'));
    await waitFor(() => {
      expect(tasksApi.delete).toHaveBeenCalledWith('t1');
    });
  });
});
```

## Change

### File: `remote-frontend/src/pages/Tasks.tsx` (EDIT — multiple anchors)

**Anchor 1 — imports block (top of file, ~L1-L7):**

Before:
```tsx
import { useLiveQuery } from '@tanstack/react-db';
import { useState } from 'react';
import {
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  createNodesCollection,
  createProjectsCollection,
  type ElectricTaskAssignment,
} from '@/lib/electric';
import { tasksApi } from '@/lib/api/tasks';
```

After:
```tsx
import { useLiveQuery } from '@tanstack/react-db';
import { useState } from 'react';
import {
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  createNodesCollection,
  createProjectsCollection,
  type ElectricTaskAssignment,
} from '@/lib/electric';
import { tasksApi } from '@/lib/api/tasks';
import { toastError, toastSuccess } from '@/lib/toast';
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
```

**Anchor 2 — component state (after the useState for selectedNodeId, ~L17):**

Before:
```tsx
  const [selectedNodeId, setSelectedNodeId] = useState<string>('');
```

After:
```tsx
  const [selectedNodeId, setSelectedNodeId] = useState<string>('');
  const [isAssigning, setIsAssigning] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
```

**Anchor 3 — handleAssign function (~L33-L35):**

Before:
```tsx
  const handleAssign = async (taskId: string) => {
    if (!selectedNodeId) return;
    try { await tasksApi.setExecutingNode(taskId, selectedNodeId); } catch (err) { console.error('setExecutingNode failed:', err); }
  };
```

After:
```tsx
  const handleAssign = async (taskId: string) => {
    if (!selectedNodeId) return;
    setIsAssigning(taskId);
    try {
      await tasksApi.setExecutingNode(taskId, selectedNodeId);
      toastSuccess('Task assigned');
    } catch (err) {
      toastError(
        err instanceof Error ? err.message : 'Assignment failed',
        { onClick: () => handleAssign(taskId) },
      );
    } finally {
      setIsAssigning(null);
    }
  };
```

**Anchor 4 — handleDelete + confirmation dialog (~L37-L39):**

Before:
```tsx
  const handleDelete = async (taskId: string) => {
    if (!confirm('Delete this task?')) return;
    try { await tasksApi.delete(taskId); } catch (err) { console.error('delete task failed:', err); }
  };
```

After:
```tsx
  const handleDelete = async (taskId: string) => {
    setDeleteTarget(taskId);
  };

  const confirmDelete = async (taskId: string) => {
    setIsDeleting(taskId);
    setDeleteTarget(null);
    try {
      await tasksApi.delete(taskId);
      toastSuccess('Task deleted');
    } catch (err) {
      toastError(
        err instanceof Error ? err.message : 'Delete failed',
        { onClick: () => confirmDelete(taskId) },
      );
    } finally {
      setIsDeleting(null);
    }
  };
```

**Anchor 5 — Assign button in the column loop (~L69-L71):**

Before:
```tsx
                  <button className="text-xs px-2 py-1 border" onClick={(e) => { e.stopPropagation(); handleAssign(a.task_id); }} aria-label="Assign">Assign</button>
```

After:
```tsx
                  <button className="text-xs px-2 py-1 border" onClick={(e) => { e.stopPropagation(); handleAssign(a.task_id); }} aria-label="Assign" disabled={isAssigning === a.task_id}>{isAssigning === a.task_id ? 'Assigning...' : 'Assign'}</button>
```

**Anchor 6 — Delete button in the column loop (~L72):**

Before:
```tsx
                  <button className="text-xs px-2 py-1 border text-red-500" onClick={(e) => { e.stopPropagation(); handleDelete(a.task_id); }} aria-label="Delete">Delete</button>
```

After:
```tsx
                  <button className="text-xs px-2 py-1 border text-red-500" onClick={(e) => { e.stopPropagation(); handleDelete(a.task_id); }} aria-label="Delete" disabled={isDeleting === a.task_id}>{isDeleting === a.task_id ? 'Deleting...' : 'Delete'}</button>
```

**Anchor 7 — add AlertDialog before the closing `</div>` of the component (~L74, right before the return statement's final `</div>`):**

Before:
```tsx
      {selectedAssignmentId && (
```

After:
```tsx
      <AlertDialog open={deleteTarget !== null} onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Are you sure?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete this task and all its assignments.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <button onClick={() => setDeleteTarget(null)} className="px-4 py-2 border rounded-md hover:bg-muted">Cancel</button>
            <button onClick={() => { if (deleteTarget) confirmDelete(deleteTarget); }} className="px-4 py-2 bg-red-500 text-white rounded-md hover:bg-red-600">Delete</button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      {selectedAssignmentId && (
```

## Allowed moves

- Edit `remote-frontend/src/pages/Tasks.tsx` at exactly the 7 anchors above. Replace each Before with its After.
- Edit `remote-frontend/src/pages/Tasks.test.tsx` to the new test code above.
- Do NOT change any other line in Tasks.tsx. The column loop structure, the STATUS_COLUMNS constant, the `byStatus` grouping logic, the live query hook calls — all stay verbatim.
- Do NOT touch the `TaskDetail` component at the bottom of the file.
- The `alert-dialog` components (`AlertDialog`, `AlertDialogContent`, etc.) already exist at `remote-frontend/src/components/ui/alert-dialog.tsx` from the shadcn base. Do NOT create or modify them.

## STOP triggers

- Any anchor line does not match exactly. The implementer MUST `git diff` before proceeding — if Tasks.tsx was modified by another concurrent task, STOP and reconcile.
- The `alert-dialog.tsx` file is missing or its exports differ from the imports used here. Verify by checking `remote-frontend/src/components/ui/alert-dialog.tsx` — it must export the 9 components listed in the imports. If any are missing (e.g., only some were ported), STOP and escalate.
- The existing Tasks.test.tsx file uses specific mocking patterns that conflict with the new test. If the existing test framework wrapper (QueryClientProvider, MemoryRouter, etc.) is needed for the new tests to pass, add it — the test code above shows the minimal version; if the test harness reveals it needs QueryClientProvider for `useLiveQuery` mocking, the implementer records this in the decisions ledger and wraps accordingly.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Tasks" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 103` exits 0.