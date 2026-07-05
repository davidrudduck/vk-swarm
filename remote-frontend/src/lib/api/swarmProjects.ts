/**
 * Swarm Projects API namespace.
 */

import type {
  CreateSwarmProjectRequest,
  LinkSwarmProjectNodeRequest,
  ListSwarmProjectNodesResponse,
  ListSwarmProjectsResponse,
  MergeSwarmProjectsRequest,
  SwarmProject,
  SwarmProjectNode,
  SwarmProjectNodeResponse,
  SwarmProjectResponse,
  SwarmProjectWithNodes,
  UpdateSwarmProjectRequest,
} from '@/types/swarm';

import { ApiError, makeRequest } from './utils';

export const swarmProjectsApi = {
  /**
   * List all swarm projects for an organization.
   */
  list: async (organizationId: string): Promise<SwarmProjectWithNodes[]> => {
    const response = await makeRequest(
      `/v1/swarm/projects?organization_id=${encodeURIComponent(organizationId)}`
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as ListSwarmProjectsResponse;
    return result.projects;
  },

  /**
   * Get a specific swarm project by ID.
   */
  getById: async (projectId: string): Promise<SwarmProject> => {
    const response = await makeRequest(`/v1/swarm/projects/${projectId}`);
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmProjectResponse;
    return result.project;
  },

  /**
   * Create a new swarm project.
   */
  create: async (data: CreateSwarmProjectRequest): Promise<SwarmProject> => {
    const response = await makeRequest('/v1/swarm/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmProjectResponse;
    return result.project;
  },

  /**
   * Update an existing swarm project.
   */
  update: async (
    projectId: string,
    data: UpdateSwarmProjectRequest
  ): Promise<SwarmProject> => {
    const response = await makeRequest(`/v1/swarm/projects/${projectId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmProjectResponse;
    return result.project;
  },

  /**
   * Delete a swarm project.
   */
  delete: async (projectId: string): Promise<void> => {
    const response = await makeRequest(`/v1/swarm/projects/${projectId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
  },

  /**
   * Merge a source swarm project into a target project.
   * The source project is deleted and all node links are transferred to the target.
   */
  merge: async (
    targetProjectId: string,
    data: MergeSwarmProjectsRequest
  ): Promise<SwarmProject> => {
    const response = await makeRequest(
      `/v1/swarm/projects/${targetProjectId}/merge`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmProjectResponse;
    return result.project;
  },

  /**
   * List all nodes linked to a swarm project.
   */
  listNodes: async (projectId: string): Promise<SwarmProjectNode[]> => {
    const response = await makeRequest(
      `/v1/swarm/projects/${projectId}/nodes`
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as ListSwarmProjectNodesResponse;
    return result.nodes;
  },

  /**
   * Link a node project to a swarm project.
   */
  linkNode: async (
    projectId: string,
    data: LinkSwarmProjectNodeRequest
  ): Promise<SwarmProjectNode> => {
    const response = await makeRequest(
      `/v1/swarm/projects/${projectId}/nodes`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmProjectNodeResponse;
    return result.link;
  },

  /**
   * Unlink a node from a swarm project.
   */
  unlinkNode: async (projectId: string, nodeId: string): Promise<void> => {
    const response = await makeRequest(
      `/v1/swarm/projects/${projectId}/nodes/${nodeId}`,
      {
        method: 'DELETE',
      }
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
  },
};
