import {
  oauthApi,
  type OAuthProvider,
  type HandoffInitResponse,
  type HandoffRedeemResponse,
} from './lib/api/oauth';

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
export const redeemOAuth = oauthApi.redeem.bind(oauthApi);

export async function getInvitation(token: string): Promise<Invitation> {
  const res = await fetch(`${API_BASE}/v1/invitations/${token}`);
  if (!res.ok) {
    throw new Error(`Invitation not found (${res.status})`);
  }
  return res.json();
}

export async function acceptInvitation(
  token: string,
  accessToken: string,
): Promise<AcceptInvitationResponse> {
  const res = await fetch(`${API_BASE}/v1/invitations/${token}/accept`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${accessToken}`,
    },
  });
  if (!res.ok) {
    throw new Error(`Failed to accept invitation (${res.status})`);
  }
  return res.json();
}
