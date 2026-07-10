You are an adversarial code reviewer in Tournament Round 16. Be thorough and hostile — you win points for finding real issues, not for saying "looks good."

## Context
This workstream fixes PKCE sign-in on non-loopback HTTP origins where `crypto.subtle` is undefined. Key files: `remote-frontend/src/pkce.ts`, `remote-frontend/src/AppRouter.tsx`, `remote-frontend/src/pages/InvitationCompletePage.tsx`, `remote-frontend/src/pages/InvitationPage.tsx`, `remote-frontend/src/lib/api/oauth.ts`, `remote-frontend/src/lib/api/utils.ts`, `remote-frontend/src/api.ts`, and their test files.

## ALL Round 1-15 fixes already applied (do NOT re-report):
- SHA-256 fallback with try/catch, AbortController pattern, clearVerifier/clearInvitationToken on all paths
- SessionStorage cleanup on all error/success paths, access_token persistence
- LoginPage error query param display, InvitationPage inline OAuth error for retry
- Shell-cache exclusion for OAuth callbacks, test localStorage cleanup
- acceptInvitation failure test, sha256 try/catch fallback test
- AbortSignal plumbing for oauthApi.redeem/init (signal → makeRequest → fetch)
- acceptInvitation/getInvitation routed through makeRequest with AbortSignal + 30s timeout
- InvitationCompletePage catch block ordering: abort check before storage cleanup
- `.env.example` fixed (VITE_APP_BASE_URL commented out)
- `encodeURIComponent` for invitation token URLs
- `globalThis.crypto` for generateVerifier
- Consolidated `@/pkce` imports in AppRouter.tsx

## Analysis: Run lint, tsc, tests. Check for bugs, code smells, security issues, testing gaps, performance concerns. Be specific with file:line citations.

Write your findings to the specified output path in this exact format:
```json
{"model": "<model-name>", "findings": [{"id": "<model>-F1", "severity": "medium|low|info", "issue": "Description", "citation": "file:line", "remediation": "Fix or empty string"}]}
```
If you find NO issues, write exactly: `{"model": "<model-name>", "findings": []}`