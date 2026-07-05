/**
 * Nodes API namespace (swarm/hive architecture).
 */

import type {
  CreateNodeApiKeyRequest,
  CreateNodeApiKeyResponse,
  MergeNodesResponse,
  Node,
  NodeApiKey,
  NodeProject,
} from '@/types/nodes';

import { ApiError, makeRequest } from './utils';

export const nodesApi = {
  list: async (organizationId: string): Promise<Node[]> => {
    const response = await makeRequest(
      `/v1/nodes?organization_id=${encodeURIComponent(organizationId)}`
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as Node[];
  },

  getById: async (nodeId: string): Promise<Node> => {
    const response = await makeRequest(`/v1/nodes/${nodeId}`);
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as Node;
  },

  delete: async (nodeId: string): Promise<void> => {
    const response = await makeRequest(`/v1/nodes/${nodeId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
  },

  listProjects: async (nodeId: string): Promise<NodeProject[]> => {
    const response = await makeRequest(`/v1/nodes/${nodeId}/projects`);
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as NodeProject[];
  },

  listApiKeys: async (organizationId: string): Promise<NodeApiKey[]> => {
    const response = await makeRequest(
      `/v1/nodes/api-keys?organization_id=${encodeURIComponent(organizationId)}`
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as NodeApiKey[];
  },

  createApiKey: async (
    data: CreateNodeApiKeyRequest
  ): Promise<CreateNodeApiKeyResponse> => {
    const response = await makeRequest('/v1/nodes/api-keys', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as CreateNodeApiKeyResponse;
  },

  revokeApiKey: async (keyId: string): Promise<void> => {
    const response = await makeRequest(`/v1/nodes/api-keys/${keyId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
  },

  /**
   * Unblock a blocked API key.
   * Requires admin access to the key's organization.
   */
  unblockApiKey: async (keyId: string): Promise<NodeApiKey> => {
    const response = await makeRequest(`/v1/nodes/api-keys/${keyId}/unblock`, {
      method: 'POST',
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as NodeApiKey;
  },

  /**
   * Merge source node into target node.
   * Moves all projects and rebinds API keys from source to target, then deletes source.
   * Requires admin access to the source node's organization.
   */
  mergeNodes: async (
    sourceNodeId: string,
    targetNodeId: string
  ): Promise<MergeNodesResponse> => {
    const response = await makeRequest(
      `/v1/nodes/${sourceNodeId}/merge-to/${targetNodeId}`,
      {
        method: 'POST',
      }
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    return await response.json() as MergeNodesResponse;
  },
};
