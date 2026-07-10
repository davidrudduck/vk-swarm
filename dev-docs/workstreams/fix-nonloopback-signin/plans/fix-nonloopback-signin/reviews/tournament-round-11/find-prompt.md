You are an adversarial code reviewer in Tournament Round 11. Your job is to find REAL bugs, code smells, and issues in the implementation of the `fix-nonloopback-signin` workstream. Be thorough and hostile — you win points for finding real issues, not for saying "looks good."

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
- `remote-frontend/vite.config.ts` — Vitest config (scripts/** excluded)
- `remote-frontend/package.json` — includes test:invariants script
- `docs/plans/fix-nonloopback-signin/decisions-ledger.md` — decisions and acceptance evidence

The spec is at `docs/superpowers/specs/2026-07-08-fix-nonloopback-signin.md`.

## Round 1 fixes already applied (do NOT re-report these):
1. `base64UrlEncode` spread overflow → replaced with loop (pkce.ts:110-118)
2. Orphaned `no-push-invariant` test → added `test:invariants` script to package.json
3. `useEffect` cleanup leak in InvitationCompletePage.tsx → added `active` flag + real cleanup return
4. Reachability gate line numbers corrected in decisions-ledger.md

## Round 2 fixes already applied (do NOT re-report these):
1. InvitationCompletePage missing params error → added error state before early return
2. InvitationCompletePage StrictMode double-effect → added `hasRun` ref
3. OAuthCallbackPage StrictMode double-effect → added `hasRun` ref
4. Invitation route test flakiness → mocked `getInvitation` and assert on heading
5. InvitationCompletePage.test.tsx error paths → added4 error path tests
6. InvitationPage.test.tsx error paths → added2 error path tests
7. SHA-256 fallback multi-block test → added FIPS 180-2 vector
8. Unnecessary string concatenation → replaced with single literals
9. rotateRight signed integers → applied `>>> 0` to return value
10. Stale-token precedence → URL token now takes precedence over sessionStorage

## Round 3 fixes already applied (do NOT re-report these):
1. OAuthCallbackPage error paths → added4 error path tests
2. isSafeReturnTo security function → exported and added6 test cases
3. LoginPage initOAuth rejection → added test case
4. InvitationPage error conflation → separated fetchError and oauthError states
5. PKCE verifier cleanup on abandonment → added clearVerifier() on early returns
6. OAuth catch block ordering → clearVerifier() before window.location.assign()
7. InvitationPage error test title → updated to match new "Sign-in failed" title

## Round 4 fixes already applied (do NOT re-report these):
1. LoginPage error query param → initialized error state from searchParams.get('error')
2. OAuth init failures stale PKCE state → added clearVerifier() and clearInvitationToken() to catch blocks
3. InvitationCompletePage access_token persistence → added localStorage.setItem('access_token', access_token)
4. InvitationCompletePage sessionStorage cleanup → added clearVerifier() and clearInvitationToken() on all early returns
5. InvitationPage sessionStorage cleanup → added clearVerifier() and clearInvitationToken() to catch block
6. LoginPage error query param test → added test case for /login?error=... display

## Round 5 fixes already applied (do NOT re-report these):
1. StrictMode interaction bug → moved storage operations BEFORE `if (!active)` guard
2. InvitationPage fetch cleanup → added `active` flag with cleanup function
3. InvitationPage error handling → added `instanceof Error` guard with fallback message
4. LoginPage stale error → updated useEffect to always set error from searchParams

## Round 6 fixes already applied (do NOT re-report these):
1. StrictMode interaction bug → replaced `hasRun` ref with `AbortController` pattern
2. OAuthCallbackPage clearInvitationToken → added clearVerifier() and clearInvitationToken() to verifier-missing path
3. Workbox runtimeCaching → added invitation routes to shell-cache

## Round 7 fixes already applied (do NOT re-report these):
1. OAuthCallbackPage invitation_token cleanup → added clearInvitationToken() to all error paths
2. OAuthCallbackPage AbortController migration → replaced hasRun ref with AbortController pattern

## Round 8 fixes already applied (do NOT re-report these):
1. OAuthCallbackPage success path clearInvitationToken → added clearInvitationToken() to success path
2. sha256 native path try/catch fallback → added try/catch to native path with fallback to sha256Fallback

## Round 9 fixes already applied (do NOT re-report these):
1. LoginPage invitation_token cleanup → added clearInvitationToken() to catch block
2. InvitationCompletePage abort check after acceptInvitation → added abort check immediately after acceptInvitation resolves

## Round 10 fixes already applied (do NOT re-report these):
1. Shell-cache OAuth callback exclusion → excluded /oauth/callback and /invitations/*/complete from shell-cache
2. InvitationCompletePage test localStorage cleanup → added localStorage.clear() to beforeEach/afterEach hooks

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