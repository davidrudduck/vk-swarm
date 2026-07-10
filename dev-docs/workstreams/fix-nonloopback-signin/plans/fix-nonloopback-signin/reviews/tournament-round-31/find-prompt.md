You are an adversarial code reviewer in Tournament Round 31. Be thorough and hostile — you win points for finding real issues, not for saying "looks good."

## Context
This workstream fixes PKCE sign-in on non-loopback HTTP origins where `crypto.subtle` is undefined. Key files: `remote-frontend/src/pkce.ts`, `remote-frontend/src/AppRouter.tsx`, `remote-frontend/src/pages/InvitationCompletePage.tsx`, `remote-frontend/src/pages/InvitationPage.tsx`, `remote-frontend/src/lib/api/oauth.ts`, `remote-frontend/src/lib/api/utils.ts`, `remote-frontend/src/api.ts`, `remote-frontend/src/components/ProfileProvider.tsx`, and their test files.

## ALL Round 1-30 fixes already applied (do NOT re-report):
- SHA-256 fallback with try/catch, AbortController pattern, clearVerifier/clearInvitationToken on all paths
- SessionStorage + localStorage cleanup on ALL error/success paths (all pages, all tests verified)
- LoginPage error query param display, InvitationPage inline OAuth error for retry
- Shell-cache exclusion for OAuth callbacks, test localStorage cleanup
- acceptInvitation failure test, sha256 try/catch fallback test
- AbortSignal plumbing for oauthApi.redeem/init (signal → makeRequest → fetch)
- acceptInvitation/getInvitation routed through makeRequest with AbortSignal + 30s timeout
- InvitationCompletePage catch block ordering: abort check before storage cleanup
- `.env.example` fixed, `encodeURIComponent` for tokens, `globalThis.crypto`, consolidated imports
- initOAuth call sites pass AbortSignal
- Error-path tests assert sessionStorage + localStorage cleanup (ALL pages, ALL tests)
- Dead AbortController removed from InvitationPage handleOAuthLogin
- InvitationPage returnTo uses encodeURIComponent(token)
- InvitationCompletePage orgSlug mismatch fixed (simplified to "Redirecting...")
- oauthApi.redeem error path test added to oauth.test.ts
- localStorage.access_token cleared on ALL error paths (OAuthCallbackPage, InvitationCompletePage, InvitationPage, LoginPage)
- ProfileProvider uses profileApi.get() with VITE_API_BASE_URL support
- ProfileProvider uses typed ApiError for 401 detection
- InvitationCompletePage error branches have complete cleanup (clearVerifier + clearInvitationToken + localStorage.removeItem)
- InvitationPage test clears localStorage in beforeEach/afterEach
- DOMException/AbortError handling in catch blocks
- invitation_token assertions in OAuth callback error-path tests
- Redirect assertion in OAuth callback success test
- makeRequest timeout error message (DOMException with user-friendly reason)
- ProfileProvider test for non-401 ApiError responses
- Double-slash URL construction fixed (trailing slashes stripped from appBase and API_BASE)
- localStorage seeded in error-path tests before asserting cleanup
- OAuth login buttons double-click prevention (loading guard)
- organizations.ts strips trailing slashes from API_BASE (R29)
- Error-path tests seed localStorage.access_token before asserting cleanup (R30)

## Known gaps (do NOT re-report these either):
- anySignal listener leak — infrastructure issue, not a code bug
- makeRequest auto-attaches Bearer token on OAuth calls — behavior change, documented
- InvitationCompletePage ignores return_to param — design decision
- StrictMode double-invokes effects — React behavior, not a bug
- initOAuth call sites don't pass AbortSignal — button click handlers, not useEffect
- encodeURIComponent test uses URL-safe token — minor testing gap
- No unit tests for utils.ts — infrastructure issue
- Playwright CI target unusable due to unrelated frontend test failures — infrastructure issue
- OAuth API error-path tests are shallow — minor testing gap
- refresh_token discarded — design decision
- OAuthCallbackPage drops safeReturnTo on error redirects — design decision
- generateVerifier assumes crypto.getRandomValues exists — design decision
- getInvitation sends auth headers to public endpoint — behavior change, documented
- profileApi.get() has no AbortSignal param — infrastructure issue
- PKCE challenge uses hex encoding instead of base64url — design decision
- makeRequest sets Content-Type on GET requests — minor issue
- OAuthCallbackPage uses oauthApi.redeem directly while InvitationCompletePage uses redeemOAuth wrapper — minor inconsistency
- ApiError exposes both status and statusCode — confusing API
- oauthApi.logout() lacks AbortSignal — minor inconsistency
- Unsafe as type assertions on response.json() — minor issue
- Redundant template literal in InvitationCompletePage — minor style issue
- Redundant useEffect in LoginPage — minor code smell
- Request timeout cleared before JSON body consumption — infrastructure issue
- isSafeReturnTo untested for same-origin absolute URLs — minor testing gap
- Component-level OAuth tests only exercise sha256Fallback path — minor testing gap
- No abort-path tests for InvitationCompletePage or OAuthCallbackPage — minor testing gap
- Dead isRedirecting state in OAuthCallbackPage — minor code smell
- Service worker shell-cache exclusion matches any path ending with /complete — minor issue
- ProfileProvider checks err.status instead of err.statusCode — minor issue
- OAuth 2.0 error_description query param silently dropped — minor UX issue
- Public OAuth handoff accepts arbitrary external return_to — security concern (backend issue)
- Same-origin return_to can re-enter OAuth callback — security concern
- Login page displays unvalidated URL error query param — security concern
- OAuth/invitation API failures discard backend JSON error body — known gap
- Double-click guard test missing — testing gap
- Redirect timer test missing — testing gap
- isSafeReturnTo end-to-end test missing — testing gap
- LoginPage calls clearInvitationToken() but never stores one — defensive cleanup
- OAuth buttons lack type="button" — minor concern
- Both buttons show "Signing in..." — minor UX concern
- Redundant instanceof DOMException checks — known code smell
- Inconsistent localStorage mocking — testing inconsistency
- InvitationCompletePage oauthError branch doesn't redirect — valid UX concern
- appBase calculation duplicated in 3 files — maintenance concern
- Double-click guard pattern duplicated — maintenance concern
- InvitationPage.test.tsx error-path test uses getAllByText which is brittle — testing concern
- setIsRedirecting(true) + window.location.assign creates flash of "Redirecting..." — UX concern
- ProfileProvider checks err.status instead of err.statusCode — code quality concern

## Analysis: Run lint, tsc, tests. Check for bugs, code smells, security issues, testing gaps, performance concerns. Be specific with file:line citations.

Write your findings to the specified output path in this exact format:
```json
{"model": "<model-name>", "findings": [{"id": "<model>-F1", "severity": "medium|low|info", "issue": "Description", "citation": "file:line", "remediation": "Fix or empty string"}]}
```
If you find NO issues, write exactly: `{"model": "<model-name>", "findings": []}`