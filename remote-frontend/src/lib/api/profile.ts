import { makeRequest, ApiError } from './utils';
import type { ProfileResponse } from '@/types/shared/types';

const API_BASE = (import.meta.env.VITE_API_BASE_URL || '').replace(/\/+$/, '');

export const profileApi = {
  async get(): Promise<ProfileResponse> {
    const accessToken = localStorage.getItem('access_token');
    if (!accessToken) {
      throw new Error('No access token found');
    }

    const response = await makeRequest(`${API_BASE}/v1/profile`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!response.ok) {
      throw new ApiError(
        `profile fetch failed: ${response.status}`,
        response.status,
        response
      );
    }
    return (await response.json()) as ProfileResponse;
  },
};
