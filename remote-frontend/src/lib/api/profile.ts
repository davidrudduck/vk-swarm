import { makeRequest, handleApiResponse } from './utils';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

export interface ProviderProfile {
  provider: string;
  username: string | null;
  display_name: string | null;
  email: string | null;
  avatar_url: string | null;
}

export interface ProfileResponse {
  user_id: string;
  username: string | null;
  email: string;
  providers: ProviderProfile[];
}

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

    return handleApiResponse<ProfileResponse>(response);
  },
};
