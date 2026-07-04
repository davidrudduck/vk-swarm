/**
 * Templates API - Template management for @mentions in descriptions.
 * Stub for hive environment (uses swarm templates instead).
 */

import type { Template } from 'shared/types';

/**
 * Templates API namespace - Stub returns empty list.
 * The hive uses swarmTemplatesApi for template management.
 */
export const templatesApi = {
  list: async (): Promise<Template[]> => {
    // Hive does not have node-local templates; return empty list
    return [];
  },

  create: async (): Promise<Template> => {
    throw new Error('Node-local templates not supported in hive environment');
  },

  update: async (): Promise<Template> => {
    throw new Error('Node-local templates not supported in hive environment');
  },

  delete: async (): Promise<void> => {
    throw new Error('Node-local templates not supported in hive environment');
  },
};
