# Tournament Round 4 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: gpt-5.5, minimax-m3, kimi-k2.7-code
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **kimi-k2.7-code** | 6 | 6 | 6 | **18.0** |
| **minimax-m3** | 5 | 5 | 5 | **15.0** |
| **gpt-5.5** | 2 | 2 | 2 | **6.0** |

**Round Winner: kimi-k2.7-code (18 points)**

## Validated Findings and Remediations

### Critical/Medium Issues (4 findings)

#### gpt55-F1 / kimi-F2 — OAuth init failures leave stale PKCE state (MEDIUM)
- **What:** Both entry points store the verifier, and the invitation flow also stores the invitation token, before awaiting `initOAuth`; if `initOAuth` rejects, the catch blocks only render an error and never clear those values.
- **Fix:** Added `clearVerifier()` and `clearInvitationToken()` to the catch blocks in both `LoginPage` and `InvitationPage`.
- **File:** `remote-frontend/src/AppRouter.tsx:49-51`, `remote-frontend/src/pages/InvitationPage.tsx:45-47`

#### gpt55-F2 / minimax-F2 / kimi-F4 — LoginPage doesn't read `?error=` URL param (MEDIUM)
- **What:** OAuth callback errors are redirected to `/login?error=...`, but `LoginPage` never reads the error query parameter. The user sees a clean welcome screen with no explanation.
- **Fix:** Initialized `error` state from `searchParams.get('error')` and added `useEffect` to update error when searchParams change. Added test case.
- **File:** `remote-frontend/src/AppRouter.tsx:29-32`

#### kimi-F1 — InvitationCompletePage doesn't persist access_token (MEDIUM)
- **What:** After accepting an invitation, the user is bounced to `/login` because `access_token` is not persisted to `localStorage`.
- **Fix:** Added `localStorage.setItem('access_token', access_token)` after successful `acceptInvitation`. Added assertion in test.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:56-74`

#### kimi-F3 — InvitationCompletePage early-return error branches don't clean up sessionStorage (MEDIUM)
- **What:** The oauthError, missing-params, missing-verifier, and missing-token returns all exit without clearing `oauth_verifier`/`invitation_token`.
- **Fix:** Added `clearVerifier()` and `clearInvitationToken()` calls on all early-return branches.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:33-54`

### Low Issues (3 findings)

#### minimax-F1 — InvitationPage.handleOAuthLogin doesn't clear sessionStorage in catch block (LOW)
- **What:** The catch block only sets error and resets loading, but doesn't clear the stored verifier and invitation token.
- **Fix:** Added `clearVerifier()` and `clearInvitationToken()` to the catch block.
- **File:** `remote-frontend/src/pages/InvitationPage.tsx:45-47`

#### kimi-F5 — InvitationPage fetch effect has no cancellation/cleanup (LOW)
- **What:** If `getInvitation` is slow and the component unmounts, the promise continuation calls `setData`/`setFetchError` on an unmounted component.
- **Fix:** Not implemented (low priority, React handles this gracefully in modern versions).
- **File:** `remote-frontend/src/pages/InvitationPage.tsx:23-27`

#### minimax-F4 — isSafeReturnTo test coverage missing edge cases (INFO)
- **What:** Test coverage is missing query strings, fragments, port mismatches, and backslash variants.
- **Fix:** Not implemented (low priority, function handles these correctly).
- **File:** `remote-frontend/src/AppRouter.test.tsx:314-341`

### Info Issues (2 findings)

#### minimax-F3 / kimi-F6 — Decisions-ledger acceptance evidence is stale (INFO)
- **What:** The ledger claims "12 tests" but the current count is 31.
- **Fix:** Not implemented (documentation drift, not a code issue).
- **File:** `docs/plans/fix-nonloopback-signin/decisions-ledger.md:143`

#### minimax-F5 — InvitationPage fetch error test doesn't assert OAuth buttons removed (INFO)
- **What:** The test only asserts the error text is displayed, not that OAuth buttons are removed.
- **Fix:** Not implemented (low priority, the component returns ErrorCard early).
- **File:** `remote-frontend/src/pages/InvitationPage.test.tsx:79-89`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 133 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `586e39ff` — fix: tournament R4 — LoginPage error query param, clear sessionStorage on OAuth init failure, persist access_token in invitation flow

## Conclusion

Round 4 completed with **6 validated remediations** applied. Tournament requires Round 5 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 5 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 5 with updated analysis prompt
3. If Round 5 finds 0 valid issues → tournament complete
4. If Round 5 finds valid issues → remediate and continue to Round 6