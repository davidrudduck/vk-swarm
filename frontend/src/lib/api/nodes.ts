/**
 * Nodes API namespace (swarm/hive architecture).
 */

import type { Node, NodeProject } from '@/types/nodes';

import { handleApiResponse, makeRequest } from './utils';

export const nodesApi = {
  list: async (organizationId: string): Promise<Node[]> => {
    const response = await makeRequest(
      `/api/nodes?organization_id=${encodeURIComponent(organizationId)}`
    );
    return handleApiResponse<Node[]>(response);
  },

  getById: async (nodeId: string): Promise<Node> => {
    const response = await makeRequest(`/api/nodes/${nodeId}`);
    return handleApiResponse<Node>(response);
  },

  delete: async (nodeId: string): Promise<void> => {
    const response = await makeRequest(`/api/nodes/${nodeId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  listProjects: async (nodeId: string): Promise<NodeProject[]> => {
    const response = await makeRequest(`/api/nodes/${nodeId}/projects`);
    return handleApiResponse<NodeProject[]>(response);
  },
};
