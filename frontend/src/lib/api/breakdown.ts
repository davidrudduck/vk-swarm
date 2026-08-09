/**
 * Task Breakdown API namespace - trigger, review, edit, accept/discard/retry breakdown proposals.
 */

import type {
  TaskBreakdownProposal,
  TaskBreakdownProposalItem,
  UpsertProposalItems,
  Task,
  TaskDependency,
} from 'shared/types';
import { makeRequest, handleApiResponse } from './utils';

export interface BreakdownWithItems {
  proposal: TaskBreakdownProposal;
  items: TaskBreakdownProposalItem[];
}

/**
 * Breakdown API namespace - task decomposition and proposal lifecycle.
 */
export const breakdownApi = {
  /**
   * Get the latest breakdown proposal (if any) and its items for a task.
   * Returns null if no proposal exists.
   */
  get: async (taskId: string): Promise<BreakdownWithItems | null> => {
    const response = await makeRequest(`/api/tasks/${taskId}/breakdown`);
    return handleApiResponse<BreakdownWithItems | null>(response);
  },

  /**
   * Trigger a new breakdown run for a task.
   * Returns the draft proposal (stage 1 synchronous).
   * Stage 2 (execution) spawns asynchronously.
   */
  trigger: async (taskId: string): Promise<TaskBreakdownProposal> => {
    const response = await makeRequest(`/api/tasks/${taskId}/breakdown`, {
      method: 'POST',
    });
    return handleApiResponse<TaskBreakdownProposal>(response);
  },

  /**
   * Replace the items in a draft proposal.
   * Only callable on Draft status proposals.
   */
  putItems: async (
    proposalId: string,
    payload: UpsertProposalItems
  ): Promise<TaskBreakdownProposalItem[]> => {
    const response = await makeRequest(
      `/api/breakdown-proposals/${proposalId}/items`,
      {
        method: 'PUT',
        body: JSON.stringify(payload),
      }
    );
    return handleApiResponse<TaskBreakdownProposalItem[]>(response);
  },

  /**
   * Accept a draft proposal, creating child tasks from its items.
   * Returns the newly created tasks.
   */
  accept: async (proposalId: string): Promise<Task[]> => {
    const response = await makeRequest(
      `/api/breakdown-proposals/${proposalId}/accept`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<Task[]>(response);
  },

  /**
   * Discard a proposal (mark as Discarded).
   * Returns the updated proposal.
   */
  discard: async (proposalId: string): Promise<TaskBreakdownProposal> => {
    const response = await makeRequest(
      `/api/breakdown-proposals/${proposalId}/discard`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<TaskBreakdownProposal>(response);
  },

  /**
   * Retry a failed proposal: creates a fresh draft and spawns a new run.
   * Returns the new proposal.
   */
  retry: async (proposalId: string): Promise<TaskBreakdownProposal> => {
    const response = await makeRequest(
      `/api/breakdown-proposals/${proposalId}/retry`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<TaskBreakdownProposal>(response);
  },

  /**
   * Get dependency edges where the given task depends on others.
   * Used to build dependency DAG visualization.
   */
  dependencies: async (taskId: string): Promise<TaskDependency[]> => {
    const response = await makeRequest(`/api/tasks/${taskId}/dependencies`);
    return handleApiResponse<TaskDependency[]>(response);
  },
};
