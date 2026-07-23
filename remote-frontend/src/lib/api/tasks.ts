import { ApiError, makeRequest } from './utils';

const API_BASE = '/v1';

/**
 * Parse a successful JSON response or throw an {@link ApiError} carrying the
 * response body and status. Consolidates the repeated `!response.ok` handling.
 */
async function parseOrThrow<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.text();
    throw new ApiError(body || 'Request failed', response.status, response);
  }
  return (await response.json()) as T;
}

/**
 * Hive `SharedTask` (bare-JSON shape from `crates/remote/src/db/tasks.rs`).
 */
export interface Task {
  id: string;
  organization_id: string;
  project_id: string | null;
  swarm_project_id: string | null;
  creator_user_id: string | null;
  assignee_user_id: string | null;
  executing_node_id: string | null;
  owner_node_id: string | null;
  owner_name: string | null;
  title: string;
  description: string | null;
  status: string;
  version: number;
  deleted_at: string | null;
  shared_at: string | null;
  archived_at: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * A minimal user summary attached to a shared task activity payload
 * (`crates/remote/src/db/users.rs::UserData`).
 */
export interface TaskUser {
  id: string;
  first_name: string | null;
  last_name: string | null;
  username: string | null;
}

/**
 * One entry of `BulkSharedTasksResponse.tasks` — the hive pairs each task
 * with its (optional) user record rather than returning a flat `Task[]`
 * (`crates/remote/src/db/tasks.rs::SharedTaskActivityPayload`).
 */
export interface TaskActivity {
  task: Task;
  user: TaskUser | null;
}

/**
 * Response shape for `GET /v1/tasks/bulk` (`crates/remote/src/routes/tasks.rs:655-659`).
 */
export interface BulkSharedTasksResponse {
  tasks: TaskActivity[];
  deleted_task_ids: string[];
  latest_seq: number | null;
}

export const tasksApi = {
  bulk: async (projectId: string): Promise<BulkSharedTasksResponse> => {
    const response = await makeRequest(
      `${API_BASE}/tasks/bulk?project_id=${encodeURIComponent(projectId)}`
    );
    return parseOrThrow<BulkSharedTasksResponse>(response);
  },

  get: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`${API_BASE}/tasks/${taskId}`);
    const result = await parseOrThrow<{ task: Task }>(response);
    return result.task;
  },

  assign: async (
    taskId: string,
    newAssigneeUserId: string | null,
    version?: number
  ): Promise<Task> => {
    const response = await makeRequest(`${API_BASE}/tasks/${taskId}/assign`, {
      method: 'POST',
      body: JSON.stringify({ new_assignee_user_id: newAssigneeUserId, version }),
    });
    const result = await parseOrThrow<{ task: Task }>(response);
    return result.task;
  },

  setExecutingNode: async (taskId: string, nodeId: string) => {
    const response = await makeRequest(`${API_BASE}/tasks/${taskId}/executing-node`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ node_id: nodeId }),
    });
    if (!response.ok) throw new Error(`setExecutingNode failed: ${response.status}`);
    return response.status === 204 ? { ok: true } : response.json();
  },

  delete: async (taskId: string) => {
    const response = await makeRequest(`${API_BASE}/tasks/${taskId}`, {
      method: 'DELETE',
    });
    if (!response.ok) throw new Error(`delete task failed: ${response.status}`);
    return response.status === 204 ? { ok: true } : response.json();
  },
};