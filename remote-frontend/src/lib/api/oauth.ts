import { makeRequest, ApiError } from './utils';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

export type OAuthProvider = 'github' | 'google';

export type HandoffInitResponse = {
  handoff_id: string;
  authorize_url: string;
};

export type HandoffRedeemResponse = {
  access_token: string;
  refresh_token: string;
};

export const oauthApi = {
  async init(
    provider: OAuthProvider,
    returnTo: string,
    appChallenge: string,
    signal?: AbortSignal
  ): Promise<HandoffInitResponse> {
    const response = await makeRequest(`${API_BASE}/v1/oauth/web/init`, {
      method: 'POST',
      body: JSON.stringify({
        provider,
        return_to: returnTo,
        app_challenge: appChallenge,
      }),
      signal,
    });

    if (!response.ok) {
      throw new ApiError(
        `oauth init failed: ${response.status}`,
        response.status,
        response
      );
    }
    return (await response.json()) as HandoffInitResponse;
  },

  async redeem(
    handoffId: string,
    appCode: string,
    appVerifier: string,
    signal?: AbortSignal
  ): Promise<HandoffRedeemResponse> {
    const response = await makeRequest(`${API_BASE}/v1/oauth/web/redeem`, {
      method: 'POST',
      body: JSON.stringify({
        handoff_id: handoffId,
        app_code: appCode,
        app_verifier: appVerifier,
      }),
      signal,
    });

    if (!response.ok) {
      throw new ApiError(
        `oauth redeem failed: ${response.status}`,
        response.status,
        response
      );
    }
    return (await response.json()) as HandoffRedeemResponse;
  },

  async logout(): Promise<void> {
    const accessToken = localStorage.getItem('access_token');
    try {
      if (accessToken) {
        await makeRequest(`${API_BASE}/v1/oauth/logout`, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${accessToken}`,
          },
        });
      }
    } catch (error) {
      console.error('Logout request failed:', error);
    } finally {
      localStorage.removeItem('access_token');
    }
  },
};
