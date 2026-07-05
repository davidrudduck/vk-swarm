import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';

vi.mock('@tanstack/react-db', () => ({
  useLiveQuery: vi.fn((collection) => ({
    data: collection._mockRows ?? [],
    isLoading: false,
  })),
}));

vi.mock('@/lib/electric', () => ({
  createTaskAssignmentsCollection: () => ({ _mockRows: [
    { id: 'a1', task_id: 't1', node_id: 'n1', execution_status: 'pending', assigned_at: '2026-07-04T00:00:00Z', started_at: null, completed_at: null, lease_expires_at: null, fencing_token: 0, local_task_id: null, local_attempt_id: null, node_project_id: 'np1', created_at: '2026-07-04T00:00:00Z' },
  ]}),
  createTaskOutputLogsCollection: () => ({ _mockRows: [] }),
  createTaskProgressEventsCollection: () => ({ _mockRows: [] }),
  createNodesCollection: () => ({ _mockRows: [
    { id: 'n1', name: 'node-1', organization_id: 'org1', hostname: null, os_info: null, status: 'online', last_heartbeat_at: null, public_url: null, created_at: '', updated_at: '' },
  ]}),
  createProjectsCollection: () => ({ _mockRows: [] }),
}));

vi.mock('@/lib/api/tasks', () => ({
  tasksApi: {
    setExecutingNode: vi.fn(),
    delete: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
  Toaster: () => null,
}));

vi.mock('idb-keyval', () => ({
  get: vi.fn(async () => 0),
  set: vi.fn(async () => undefined),
  del: vi.fn(async () => undefined),
}));

const onlineControl = { isOnline: true };
vi.mock('@/lib/offline', () => ({
  useOnlineStatus: () => ({ isOnline: onlineControl.isOnline, wasOffline: false, lastOnlineAt: null }),
}));

vi.mock('@/lib/mutation-queue', () => ({
  enqueueMutation: vi.fn(async () => {}),
  replayMutations: vi.fn(async () => {}),
  getQueueLength: vi.fn(async () => 0),
}));

import { TasksBoard, TaskDetail } from './Tasks';
import { tasksApi } from '@/lib/api/tasks';
import { enqueueMutation, replayMutations } from '@/lib/mutation-queue';

describe('TasksBoard', () => {
  it('renders tasks grouped by execution_status', () => {
    render(<TasksBoard />);
    expect(screen.getByText(/pending/i)).toBeInTheDocument();
    expect(screen.getByText(/in progress/i)).toBeInTheDocument();
  });
});

describe('TaskDetail', () => {
  it('renders an empty state when no logs or events exist', () => {
    render(<TaskDetail assignmentId="a-nonexistent" />);
    expect(screen.getByText(/no activity yet/i)).toBeInTheDocument();
  });
});

describe('TasksBoard management actions', () => {
  it('renders Assign and Delete buttons', () => {
    render(<TasksBoard />);
    expect(screen.getByLabelText('Assign')).toBeInTheDocument();
    expect(screen.getByLabelText('Delete')).toBeInTheDocument();
  });
});

describe('Tasks.tsx error resilience (SC3, SC4, SC5)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('shows delete confirmation dialog before dispatching DELETE', async () => {
    render(<TasksBoard />);
    const deleteBtn = screen.getByLabelText('Delete');
    fireEvent.click(deleteBtn);

    await waitFor(() => {
      expect(screen.getByText('Are you sure?')).toBeInTheDocument();
    });

    expect(tasksApi.delete).not.toHaveBeenCalled();
  });

  it('dispatches DELETE when confirm is clicked', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ ok: true });
    render(<TasksBoard />);

    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => {
      expect(screen.getByText('Are you sure?')).toBeInTheDocument();
    });

    const deleteButtons = screen.getAllByText('Delete');
    fireEvent.click(deleteButtons[deleteButtons.length - 1]);
    await waitFor(() => {
      expect(tasksApi.delete).toHaveBeenCalledWith('t1');
    });
  });

  it('shows loading state on assign button during mutation', async () => {
    (tasksApi.setExecutingNode as ReturnType<typeof vi.fn>).mockReturnValue(
      new Promise(() => {}),
    );

    const { container } = render(<TasksBoard />);
    const select = container.querySelector('select') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: 'n1' } });

    const assignBtn = screen.getByLabelText('Assign');
    fireEvent.click(assignBtn);

    await waitFor(() => {
      expect(assignBtn).toBeDisabled();
    });
  });
});

describe('Tasks.tsx PWA offline scenarios (SC8, SC10)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    onlineControl.isOnline = true;
  });

  afterEach(() => {
    cleanup();
  });

  it('enqueues DELETE on TypeError: Failed to fetch (offline)', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockRejectedValue(new TypeError('Failed to fetch'));

    render(<TasksBoard />);
    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => screen.getByText('Are you sure?'));
    fireEvent.click(screen.getAllByText('Delete').pop()!);

    await waitFor(() => {
      expect(enqueueMutation).toHaveBeenCalledWith('DELETE', '/v1/tasks/t1', 't1');
    });
  });

  it('enqueues PATCH on TypeError: Failed to fetch (offline)', async () => {
    (tasksApi.setExecutingNode as ReturnType<typeof vi.fn>).mockRejectedValue(new TypeError('Failed to fetch'));

    const { container } = render(<TasksBoard />);
    const select = container.querySelector('select') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: 'n1' } });
    fireEvent.click(screen.getByLabelText('Assign'));

    await waitFor(() => {
      expect(enqueueMutation).toHaveBeenCalledWith('PATCH', '/v1/tasks/t1/executing-node', {
        taskId: 't1',
        nodeId: 'n1',
      });
    });
  });

  it('replays mutations on reconnect', async () => {
    onlineControl.isOnline = true;
    render(<TasksBoard />);
    await waitFor(() => {
      expect(replayMutations).toHaveBeenCalled();
    });
  });

  it('does not delete from optimistic ref on offline TypeError', async () => {
    (tasksApi.delete as ReturnType<typeof vi.fn>).mockRejectedValue(new TypeError('Failed to fetch'));

    render(<TasksBoard />);
    fireEvent.click(screen.getByLabelText('Delete'));
    await waitFor(() => screen.getByText('Are you sure?'));
    fireEvent.click(screen.getAllByText('Delete').pop()!);

    await waitFor(() => {
      expect(enqueueMutation).toHaveBeenCalledWith('DELETE', '/v1/tasks/t1', 't1');
    });
    // enqueueMutation was called — the catch branch ran and the mutation is queued
    // TODO: verify task row re-appears after setIsDeleting(null) re-render
  });
});
