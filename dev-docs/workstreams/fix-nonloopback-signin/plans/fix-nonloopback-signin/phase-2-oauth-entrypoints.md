# Phase 2 — OAuth entrypoint regressions

Prove the fixed PKCE primitive reaches real user routes and preserves storage behavior. This phase is shippable when normal login, OAuth callback redemption, invitation OAuth start, and invitation completion storage are all covered by route-level tests.

Tasks:

- `201` — Cover non-loopback normal login and callback storage.
- `202` — Cover non-loopback invitation OAuth and completion storage.

Exit criteria:

- `/login` reaches `initOAuth()` with a 64-character lowercase hex challenge when `crypto.subtle` is absent.
- `/oauth/callback` still redeems with the stored verifier and clears it.
- `/invitations/:token/accept` reaches `initOAuth()` with the invitation return URL and stores both verifier and invitation token.
- Invitation completion still redeems with the stored verifier and accepts with the stored invitation token.
