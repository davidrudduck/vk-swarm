import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('@tanstack/react-db', () => ({
  useLiveQuery: vi.fn((collection) => ({
    data: collection._mockRows ?? [],
    isLoading: false,
  })),
}));

vi.mock('@/lib/electric', () => ({
  createTaskAssignmentsCollection: () => ({ _mockRows: [
    { id: 'a1', task_id: 't1', node_id: 'n1', execution_status: 'in_progress', assigned_at: '2026-07-04T00:00:00Z', started_at: '2026-07-04T00:00:00Z', completed_at: null, lease_expires_at: null, fencing_token: 1, local_task_id: null, local_attempt_id: null, node_project_id: 'np1', created_at: '2026-07-04T00:00:00Z' },
    { id: 'a2', task_id: 't2', node_id: 'n2', execution_status: 'pending', assigned_at: '2026-07-04T00:00:00Z', started_at: null, completed_at: null, lease_expires_at: null, fencing_token: 0, local_task_id: null, local_attempt_id: null, node_project_id: 'np2', created_at: '2026-07-04T00:00:00Z' },
  ]}),
  createTaskOutputLogsCollection: () => ({ _mockRows: [] }),
  createTaskProgressEventsCollection: () => ({ _mockRows: [] }),
  createNodesCollection: () => ({ _mockRows: [
    { id: 'n1', name: 'node-alpha', organization_id: 'org1', hostname: null, os_info: null, status: 'online', last_heartbeat_at: null, public_url: null, created_at: '', updated_at: '' },
    { id: 'n2', name: 'node-beta', organization_id: 'org1', hostname: null, os_info: null, status: 'online', last_heartbeat_at: null, public_url: null, created_at: '', updated_at: '' },
  ]}),
  createProjectsCollection: () => ({ _mockRows: [] }),
}));

import { TasksBoard } from './Tasks';

describe('TasksBoard', () => {
  it('renders tasks grouped by execution_status across multiple nodes', () => {
    render(<TasksBoard />);
    expect(screen.getByText(/pending/i)).toBeInTheDocument();
    expect(screen.getByText(/in progress/i)).toBeInTheDocument();
    expect(screen.getByText('node-alpha')).toBeInTheDocument();
    expect(screen.getByText('node-beta')).toBeInTheDocument();
  });
});