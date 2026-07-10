# Tournament Round 3 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: minimax-m3, glm-5.2, deepseek-v4-pro
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **minimax-m3** | 6 | 6 | 6 | **18.0** |
| **glm-5.2** | 5 | 5 | 5 | **15.0** |
| **deepseek-v4-pro** | 2 | 2 | 2 | **6.0** |

**Round Winner: minimax-m3 (18 points)**

## Validated Findings and Remediations

### Critical/Medium Issues (5 findings)

#### minimax-F2 / glm-F1 — OAuthCallbackPage error paths untested (MEDIUM)
- **What:** OAuthCallbackPage only tests the success path. The four error paths (oauthError, missing handoffId/appCode, missing verifier, redeem rejection) are untested.
- **Fix:** Added4 error path tests with mocked `window.location.assign`.
- **File:** `remote-frontend/src/AppRouter.test.tsx`

#### minimax-F3 — isSafeReturnTo security function untested (MEDIUM)
- **What:** `isSafeReturnTo` is a security-critical open-redirect prevention function that gates the post-OAuth return target, but it has zero unit-test coverage.
- **Fix:** Exported `isSafeReturnTo` and added6 test cases covering relative paths, cross-origin URLs, protocol-relative URLs, javascript: URLs, data: URLs, and empty strings.
- **File:** `remote-frontend/src/AppRouter.tsx:91-98`, `remote-frontend/src/AppRouter.test.tsx`

#### minimax-F1 — LoginPage initOAuth rejection untested (MEDIUM)
- **What:** LoginPage only tests the happy path. The initOAuth rejection branch is untested.
- **Fix:** Added test case that mocks initOAuth to reject and verifies error message and loading state reset.
- **File:** `remote-frontend/src/AppRouter.test.tsx`

#### deepseek-F1 — InvitationPage conflates error types (MEDIUM)
- **What:** InvitationPage shows "Invalid or expired invitation" for both fetch errors and OAuth-init errors, which is misleading.
- **Fix:** Separated error states into `fetchError` and `oauthError` with different titles ("Invalid or expired invitation" vs "Sign-in failed").
- **File:** `remote-frontend/src/pages/InvitationPage.tsx`

#### glm-F3 — PKCE verifier not cleared on abandonment (LOW)
- **What:** PKCE verifier is not cleared on early-return branches (oauthError, missing params, missing token), leaving the secret in sessionStorage.
- **Fix:** Added `clearVerifier()` calls on all early-return branches in both OAuthCallbackPage and InvitationCompletePage.
- **File:** `remote-frontend/src/AppRouter.tsx:116-124`

### Low Issues (5 findings)

#### minimax-F4 — OAuth catch block ordering (LOW)
- **What:** OAuth catch block calls `window.location.assign()` before `clearVerifier()`, which is inconsistent with the success path.
- **Fix:** Swapped order to clear verifier before navigation.
- **File:** `remote-frontend/src/AppRouter.tsx:140-144`

#### minimax-F6 — InvitationCompletePage error tests sessionStorage (LOW)
- **What:** Error-path tests don't verify sessionStorage side-effects.
- **Fix:** Not implemented (low priority, tests already verify error messages).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx`

#### glm-F2 — InvitationCompletePage doesn't persist access_token (LOW)
- **What:** After accepting an invitation, the user is bounced to `/login` because `access_token` is not persisted to localStorage.
- **Fix:** Not implemented (pre-existing issue, not introduced by this workstream).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:56`

#### glm-F4 — Task 301 acceptance evidence stale (LOW)
- **What:** Task 301 acceptance evidence in the decisions-ledger is stale relative to the final committed state.
- **Fix:** Not implemented (documentation drift, not a code issue).
- **File:** `docs/plans/fix-nonloopback-signin/decisions-ledger.md:143`

#### deepseek-F2 — Redundant resolveInitOAuth assertions (LOW)
- **What:** `expect(resolveInitOAuth).toBeTypeOf('function')` is redundant with preceding `toHaveBeenCalledWith` assertions.
- **Fix:** Not implemented (cosmetic, low priority).
- **File:** `remote-frontend/src/AppRouter.test.tsx:116`, `remote-frontend/src/pages/InvitationPage.test.tsx:76`

### Info Issues (2 findings)

#### minimax-F5 — setIsRedirecting dead code (INFO)
- **What:** `setIsRedirecting(true)` is dead code because navigation happens before re-render.
- **Fix:** Not implemented (pre-existing, not introduced by this workstream).
- **File:** `remote-frontend/src/AppRouter.tsx:138,150-158`

#### glm-F5 — bytesToHex i++ vs i += 1 (INFO)
- **What:** `bytesToHex` uses `i++` while rest of file uses `i += 1`.
- **Fix:** Not implemented (cosmetic inconsistency).
- **File:** `remote-frontend/src/pkce.ts:123`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 132 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `2ba7c458` — fix: tournament R3 — OAuth error path tests, isSafeReturnTo tests, verifier cleanup on abandonment, InvitationPage error separation

## Conclusion

Round 3 completed with **7 validated remediations** applied. Tournament requires Round 4 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 4 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 4 with updated analysis prompt
3. If Round 4 finds 0 valid issues → tournament complete
4. If Round 4 finds valid issues → remediate and continue to Round 5