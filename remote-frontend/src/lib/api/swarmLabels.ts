/**
 * Swarm Labels API namespace.
 */

import type {
  CreateSwarmLabelRequest,
  ListSwarmLabelsResponse,
  MergeSwarmLabelsRequest,
  PromoteLabelToOrgRequest,
  SwarmLabel,
  SwarmLabelResponse,
  UpdateSwarmLabelRequest,
} from '@/types/swarm';

import { ApiError, makeRequest } from './utils';

export const swarmLabelsApi = {
  /**
   * List all swarm labels for an organization.
   */
  list: async (organizationId: string): Promise<SwarmLabel[]> => {
    const response = await makeRequest(
      `/v1/swarm/labels?organization_id=${encodeURIComponent(organizationId)}`
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as ListSwarmLabelsResponse;
    return result.labels;
  },

  /**
   * Get a specific swarm label by ID.
   */
  getById: async (labelId: string): Promise<SwarmLabel> => {
    const response = await makeRequest(`/v1/swarm/labels/${labelId}`);
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmLabelResponse;
    return result.label;
  },

  /**
   * Create a new organization-global swarm label.
   */
  create: async (data: CreateSwarmLabelRequest): Promise<SwarmLabel> => {
    const response = await makeRequest('/v1/swarm/labels', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmLabelResponse;
    return result.label;
  },

  /**
   * Update an existing swarm label.
   */
  update: async (
    labelId: string,
    data: UpdateSwarmLabelRequest
  ): Promise<SwarmLabel> => {
    const response = await makeRequest(`/v1/swarm/labels/${labelId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmLabelResponse;
    return result.label;
  },

  /**
   * Delete a swarm label.
   */
  delete: async (labelId: string): Promise<void> => {
    const response = await makeRequest(`/v1/swarm/labels/${labelId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
  },

  /**
   * Merge a source label into a target label.
   * All tasks with the source label are updated to use the target label.
   */
  merge: async (
    targetLabelId: string,
    data: MergeSwarmLabelsRequest
  ): Promise<SwarmLabel> => {
    const response = await makeRequest(
      `/v1/swarm/labels/${targetLabelId}/merge`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmLabelResponse;
    return result.label;
  },

  /**
   * Promote a project-specific label to an organization-global label.
   */
  promoteToOrg: async (data: PromoteLabelToOrgRequest): Promise<SwarmLabel> => {
    const response = await makeRequest('/v1/swarm/labels/promote', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmLabelResponse;
    return result.label;
  },
};
