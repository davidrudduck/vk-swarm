---
id: "205"
phase: 2
title: Wire optimistic mutations + offline queue into Tasks.tsx
status: ready
depends_on: ["103", "202", "204"]
parallel: false
conflicts_with: [103]
files:
  - remote-frontend/src/pages/Tasks.tsx
  - remote-frontend/src/pages/Tasks.test.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Tasks"
allowed_change: edit
covers_criteria: [SC8, SC10]
---

## Failing test (write first)

Update `remote-frontend/src/pages/Tasks.test.tsx` (already modified in task 103) with these additional test cases:

Append to the existing file:
```tsx
import { enqueueMutation, replayMutations } from '@/lib/mutation-queue';

vi.mock('@/lib/mutation-queue', () => ({
  enqueueMutation: vi.fn(),
  getQueueLength: vi.fn(() => Promise.resolve(0)),
  replayMutations: vi.fn(),
}));

vi.mock('@/lib/offline', () => ({
  useOnlineStatus: vi.fn(() => ({ isOnline: true, wasOffline: false, lastOnlineAt: null })),
}));

describe('Tasks.tsx PWA features (SC8, SC10)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedUseLiveQuery.mockReturnValue({ data: [] });
  });

  it('hides deleted task immediately (optimistic removal)', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ ok: true });

    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
      ],
    });

    render(<TasksBoard />);

    expect(screen.getByText('task t1')).toBeDefined();

    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => screen.getByText('Are you sure?'));
    fireEvent.click(screen.getByText('Delete'));

    await waitFor(() => {
      expect(screen.queryByText('task t1')).toBeNull();
    });
  });

  it('restores deleted task on API error', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Server error'));

    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
      ],
    });

    render(<TasksBoard />);

    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => screen.getByText('Are you sure?'));
    fireEvent.click(screen.getByText('Delete'));

    await waitFor(() => {
      expect(screen.getByText('task t1')).toBeDefined();
    });
  });

  it('enqueues mutation on network error', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockRejectedValue(new TypeError('Failed to fetch'));

    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
      ],
    });

    render(<TasksBoard />);
    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => screen.getByText('Are you sure?'));
    fireEvent.click(screen.getByText('Delete'));

    await waitFor(() => {
      expect(enqueueMutation).toHaveBeenCalled();
    });
  });

  it('replays queued mutations when coming online', async () => {
    const replayMock = replayMutations as ReturnType<typeof vi.fn>;
    const { useOnlineStatus } = await import('@/lib/offline');
    (useOnlineStatus as ReturnType<typeof vi.fn>).mockReturnValue({
      isOnline: true,
      wasOffline: true,
      lastOnlineAt: new Date(),
    });

    replayMock.mockResolvedValue(undefined);
    mockedUseLiveQuery.mockReturnValue({
      data: [
        { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
        { id: 'n1', name: 'node-1' },
      ],
    });

    render(<TasksBoard />);

    await waitFor(() => {
      expect(replayMutations).toHaveBeenCalled();
    });
  });
});
```

## Change

### File: `remote-frontend/src/pages/Tasks.tsx` (EDIT — five anchors)

The board data comes from `useLiveQuery` (TanStack DB), NOT React Query cache. The `optimisticUpdate`/`optimisticDelete` helpers from task 202 operate on React Query's `queryClient`, which is a separate cache. Therefore, optimistic mutations use LOCAL React state (a `useRef<Set>` for deletions) instead of the task 202 helpers. The `replayMutations`/`enqueueMutation` from task 204 are still used for offline support.

**Anchor 1 — imports (builds on task 103's After state):**

Before (from task 103):
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

After:
```tsx
import { useLiveQuery } from '@tanstack/react-db';
import { useState, useRef, useEffect, useCallback } from 'react';
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
import { enqueueMutation, replayMutations } from '@/lib/mutation-queue';
import { useOnlineStatus } from '@/lib/offline';
```

**Anchor 2 — component body (after `const nodeNames = new Map(...)`):**

After the line:
```tsx
  const nodeNames = new Map(nodes.map((n: { id: string; name: string }) => [n.id, n.name]));
```
Add:
```tsx
  const optimisticRemovals = useRef<Set<string>>(new Set());
  const { isOnline } = useOnlineStatus();

  const replayPending = useCallback(async () => {
    await replayMutations(
      async (entry) => {
        if (entry.operation === 'DELETE') {
          await tasksApi.delete(entry.payload as string);
        } else if (entry.operation === 'PATCH') {
          const { taskId, nodeId } = entry.payload as { taskId: string; nodeId: string };
          await tasksApi.setExecutingNode(taskId, nodeId);
        }
      },
      (_entry, err) => {
        toastError(`Queued mutation failed: ${err.message}`, {
          onClick: () => replayPending(),
        });
      },
    );
  }, []);

  useEffect(() => {
    if (isOnline) {
      replayPending();
    }
  }, [isOnline, replayPending]);
```

**Anchor 3 — handleAssign (replace from task 103):**

Before (from task 103):
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

After:
```tsx
  const handleAssign = async (taskId: string) => {
    if (!selectedNodeId) return;
    setIsAssigning(taskId);
    try {
      await tasksApi.setExecutingNode(taskId, selectedNodeId);
      toastSuccess('Task assigned');
    } catch (err) {
      if (err instanceof TypeError && err.message === 'Failed to fetch') {
        await enqueueMutation('PATCH', `/v1/tasks/${taskId}/executing-node`, {
          taskId,
          nodeId: selectedNodeId,
        });
        toastSuccess('Assignment queued for sync');
      } else {
        toastError(
          err instanceof Error ? err.message : 'Assignment failed',
          { onClick: () => handleAssign(taskId) },
        );
      }
    } finally {
      setIsAssigning(null);
    }
  };
```

**Anchor 4 — confirmDelete (replace from task 103):**

Before (from task 103):
```tsx
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

After:
```tsx
  const confirmDelete = async (taskId: string) => {
    setIsDeleting(taskId);
    setDeleteTarget(null);
    optimisticRemovals.current.add(taskId);
    try {
      await tasksApi.delete(taskId);
      toastSuccess('Task deleted');
    } catch (err) {
      optimisticRemovals.current.delete(taskId);
      if (err instanceof TypeError && err.message === 'Failed to fetch') {
        await enqueueMutation('DELETE', `/v1/tasks/${taskId}`, taskId);
        toastSuccess('Deletion queued for sync');
      } else {
        toastError(
          err instanceof Error ? err.message : 'Delete failed',
          { onClick: () => confirmDelete(taskId) },
        );
      }
    } finally {
      setIsDeleting(null);
    }
  };
```

**Anchor 5 — render loop filter (the `byStatus` grouping block):**

Before:
```tsx
  const byStatus = new Map<string, ElectricTaskAssignment[]>();
  for (const status of STATUS_COLUMNS) byStatus.set(status, []);
  for (const a of assignments) {
    const bucket = byStatus.get(a.execution_status) ?? byStatus.get('pending');
    bucket?.push(a);
  }
```

After:
```tsx
  const byStatus = new Map<string, ElectricTaskAssignment[]>();
  for (const status of STATUS_COLUMNS) byStatus.set(status, []);
  for (const a of assignments) {
    if (optimisticRemovals.current.has(a.id)) continue;
    const bucket = byStatus.get(a.execution_status) ?? byStatus.get('pending');
    bucket?.push(a);
  }
```

## Allowed moves

- Edit `remote-frontend/src/pages/Tasks.tsx` at exactly the 5 anchors above.
- Update `remote-frontend/src/pages/Tasks.test.tsx` to add the PWA test cases above.
- Do NOT change any line outside the identified anchors.
- Do NOT call `optimisticUpdate` or `optimisticDelete` from task 202 — the board uses `useLiveQuery` data which those helpers cannot affect.
- Do NOT touch any other file.

## STOP triggers

- Task 103 is not complete (Tasks.tsx doesn't have the task 103 After state). The Before anchors will not match. Verify with `git diff`.
- `replayMutations` from task 204 has a different signature. Verify `remote-frontend/src/lib/mutation-queue.ts` exports match.
- `useOnlineStatus` from task 201 has a different signature. Verify.
- SC4 guard fails: `cd ../frontend && npx tsc --noEmit` exits non-zero.

## Design note: local state overlay vs task 202 helpers

The task 202 optimistic helpers operate on `@tanstack/react-query`'s `QueryClient` cache. But `Tasks.tsx` reads board data via `@tanstack/react-db`'s `useLiveQuery` which has its own isolated cache. The two caches are separate. A `queryClient.setQueryData` call does not affect what `useLiveQuery` returns.

This task uses a `useRef<Set<string>>` as a local optimistic overlay: deleted items are filtered out of the live query results before rendering. On API success, the live query eventually syncs the backend deletion. On API error, the item is restored to the view. This is the simplest correct approach for the current data-loading architecture.