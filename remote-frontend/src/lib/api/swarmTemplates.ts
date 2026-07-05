/**
 * Swarm Templates API namespace.
 */

import type {
  CreateSwarmTemplateRequest,
  ListSwarmTemplatesResponse,
  MergeSwarmTemplatesRequest,
  SwarmTemplate,
  SwarmTemplateResponse,
  UpdateSwarmTemplateRequest,
} from '@/types/swarm';

import { ApiError, makeRequest } from './utils';

export const swarmTemplatesApi = {
  /**
   * List all swarm templates for an organization.
   */
  list: async (organizationId: string): Promise<SwarmTemplate[]> => {
    const response = await makeRequest(
      `/v1/swarm/templates?organization_id=${encodeURIComponent(organizationId)}`
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as ListSwarmTemplatesResponse;
    return result.templates;
  },

  /**
   * Get a specific swarm template by ID.
   */
  getById: async (templateId: string): Promise<SwarmTemplate> => {
    const response = await makeRequest(`/v1/swarm/templates/${templateId}`);
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmTemplateResponse;
    return result.template;
  },

  /**
   * Create a new swarm template.
   */
  create: async (data: CreateSwarmTemplateRequest): Promise<SwarmTemplate> => {
    const response = await makeRequest('/v1/swarm/templates', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmTemplateResponse;
    return result.template;
  },

  /**
   * Update an existing swarm template.
   */
  update: async (
    templateId: string,
    data: UpdateSwarmTemplateRequest
  ): Promise<SwarmTemplate> => {
    const response = await makeRequest(`/v1/swarm/templates/${templateId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmTemplateResponse;
    return result.template;
  },

  /**
   * Delete a swarm template.
   */
  delete: async (templateId: string): Promise<void> => {
    const response = await makeRequest(`/v1/swarm/templates/${templateId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
  },

  /**
   * Merge a source template into a target template.
   */
  merge: async (
    targetTemplateId: string,
    data: MergeSwarmTemplatesRequest
  ): Promise<SwarmTemplate> => {
    const response = await makeRequest(
      `/v1/swarm/templates/${targetTemplateId}/merge`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    if (!response.ok) {
      const body = await response.text();
      throw new ApiError(body || 'Request failed', response.status, response);
    }
    const result = await response.json() as SwarmTemplateResponse;
    return result.template;
  },
};
