You are an adversarial code reviewer in Tournament Round 14. Your job is to find REAL bugs, code smells, and issues in the implementation of the `fix-nonloopback-signin` workstream. Be thorough and hostile — you win points for finding real issues, not for saying "looks good."

## Context

This workstream fixes PKCE sign-in on non-loopback HTTP origins where `crypto.subtle` is undefined. The implementation adds a SHA-256 fallback in `remote-frontend/src/pkce.ts` and route-level tests for `/login` and `/invitations/:token/accept`.

Key files to review:
- `remote-frontend/src/pkce.ts` — the PKCE implementation with SHA-256 fallback
- `remote-frontend/src/pkce.test.ts` — unit tests for PKCE
- `remote-frontend/src/AppRouter.test.tsx` — login + callback route tests
- `remote-frontend/src/pages/InvitationPage.test.tsx` — invitation OAuth test
- `remote-frontend/src/pages/InvitationCompletePage.test.tsx` — invitation completion test
- `remote-frontend/src/pages/InvitationCompletePage.tsx` — invitation completion component
- `remote-frontend/src/pages/InvitationPage.tsx` — invitation page with separate error states
- `remote-frontend/src/AppRouter.tsx` — OAuth callback page with isSafeReturnTo
- `remote-frontend/src/lib/api/oauth.ts` — OAuth API with AbortSignal support
- `remote-frontend/src/api.ts` — API re-exports including redeemOAuth
- `remote-frontend/vite.config.ts` — Vitest config (scripts/** excluded)
- `remote-frontend/package.json` — includes test:invariants script
- `docs/plans/fix-nonloopback-signin/decisions-ledger.md` — decisions and acceptance evidence

The spec is at `docs/superpowers/specs/2026-07-08-fix-nonloopback-signin.md`.

## ALL fixes already applied (do NOT re-report these):
Round 1-12 fixes are documented in the decisions-ledger. Key fixes include:
- SHA-256 fallback with try/catch, AbortController pattern, clearVerifier/clearInvitationToken on all paths
- SessionStorage cleanup on all error/success paths, access_token persistence
- LoginPage error query param display, InvitationPage inline OAuth error for retry
- Shell-cache exclusion for OAuth callbacks, test localStorage cleanup
- acceptInvitation failure test, sha256 try/catch fallback test

## Round 13 fixes already applied (do NOT re-report these):
1. AbortSignal plumbing → added `signal` parameter to `oauthApi.redeem` and `redeemOAuth`, threaded through to `makeRequest`
2. OAuthCallbackPage and InvitationCompletePage now pass `abortController.signal` to redeem calls

## Analysis: Run lint, tsc, tests. Check for bugs, code smells, security issues, testing gaps, performance concerns. Be specific with file:line citations.

## Deliverable
Write your findings to the specified output path in this exact format:
```json
{
  "model": "<model-name>",
  "findings": [
    {
      "id": "<model>-F1",
      "severity": "medium|low|info",
      "issue": "Description of the issue",
      "citation": "file:line",
      "remediation": "Concrete fix description or empty string"
    }
  ]
}
```

If you find NO issues, write exactly: `{"model": "<model-name>", "findings": []}`