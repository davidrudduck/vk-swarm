import { makeRequest } from './utils';

const API_BASE = '/v1';

export const tasksApi = {
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