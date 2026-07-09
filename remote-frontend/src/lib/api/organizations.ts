import { makeRequest, ApiError } from './utils';
import type { Organization } from '@/types/shared/types';

const API_BASE = (import.meta.env.VITE_API_BASE_URL || '').replace(/\/+$/, '');

export const organizationsApi = {
  async list(): Promise<Organization[]> {
    const accessToken = localStorage.getItem('access_token');
    if (!accessToken) {
      throw new Error('No access token found');
    }

    const response = await makeRequest(`${API_BASE}/v1/organizations`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!response.ok) {
      throw new ApiError(
        `organizations list failed: ${response.status}`,
        response.status,
        response
      );
    }

    const data = (await response.json()) as { organizations: Organization[] };
    return data.organizations;
  },
};
