/**
 * API module - Re-exports from API submodules.
 */

// Utils
export {
  ApiError,
  REQUEST_TIMEOUT_MS,
  anySignal,
  makeRequest,
} from './utils';

// Nodes API (swarm/hive architecture)
export { nodesApi } from './nodes';

// Organizations API
export { organizationsApi } from './organizations';

// Swarm Projects API
export { swarmProjectsApi } from './swarmProjects';

// Swarm Labels API
export { swarmLabelsApi } from './swarmLabels';

// Templates API (stub for hive)
export { templatesApi } from './templates';

// Swarm Templates API
export { swarmTemplatesApi } from './swarmTemplates';
