import type { BrowserAuthState } from 'shared/types';

import { handleApiResponse, makeRequest } from './utils';

export const browserAuthApi = {
  getState: async (): Promise<BrowserAuthState> => {
    const response = await makeRequest('/api/auth/state');
    return handleApiResponse<BrowserAuthState>(response);
  },
  startLogin: async (
    provider: string,
    returnTo: string
  ): Promise<{ handoff_id: string; authorize_url: string }> => {
    const response = await makeRequest('/api/auth/handoff/init', {
      method: 'POST',
      body: JSON.stringify({ provider, return_to: returnTo }),
    });
    return handleApiResponse(response);
  },
  logout: async (): Promise<void> => {
    const response = await makeRequest('/api/auth/browser/logout', {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },
  disconnectHive: async (): Promise<void> => {
    const response = await makeRequest('/api/auth/logout', { method: 'POST' });
    await handleApiResponse<void>(response);
  },
};
