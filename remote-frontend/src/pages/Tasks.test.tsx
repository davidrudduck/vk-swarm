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

vi.mock('@/lib/offline', () => ({
  useOnlineStatus: () => ({ isOnline: true, wasOffline: false, lastOnlineAt: null }),
}));

import { TasksBoard, TaskDetail } from './Tasks';
import { tasksApi } from '@/lib/api/tasks';

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
