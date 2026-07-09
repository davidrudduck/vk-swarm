import {
  oauthApi,
  type OAuthProvider,
  type HandoffInitResponse,
  type HandoffRedeemResponse,
} from './lib/api/oauth';
import { makeRequest, ApiError } from './lib/api/utils';

const API_BASE = import.meta.env.VITE_API_BASE_URL || "";

export type Invitation = {
  id: string;
  organization_slug: string;
  organization_name: string;
  role: string;
  expires_at: string;
};

export type AcceptInvitationResponse = {
  organization_id: string;
  organization_slug: string;
  role: string;
};

// Re-export OAuth types for backwards compatibility
export type { OAuthProvider, HandoffInitResponse, HandoffRedeemResponse };

// Re-export OAuth functions for backwards compatibility
export const initOAuth = oauthApi.init.bind(oauthApi);
export const redeemOAuth = (
  handoffId: string,
  appCode: string,
  appVerifier: string,
  signal?: AbortSignal
) => oauthApi.redeem(handoffId, appCode, appVerifier, signal);

export async function getInvitation(token: string, signal?: AbortSignal): Promise<Invitation> {
  const res = await makeRequest(`${API_BASE}/v1/invitations/${encodeURIComponent(token)}`, { signal });
  if (!res.ok) {
    throw new ApiError(`Invitation not found (${res.status})`, res.status, res);
  }
  return res.json();
}

export async function acceptInvitation(
  token: string,
  accessToken: string,
  signal?: AbortSignal,
): Promise<AcceptInvitationResponse> {
  const res = await makeRequest(`${API_BASE}/v1/invitations/${encodeURIComponent(token)}/accept`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
    signal,
  });
  if (!res.ok) {
    throw new ApiError(`Failed to accept invitation (${res.status})`, res.status, res);
  }
  return res.json();
}
