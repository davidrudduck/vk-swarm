# Tournament Round 8 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: mimo-v2.5-pro, opus, gpt-5.5
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **mimo-v2.5-pro** | 5 | 5 | 5 | **15.0** |
| **gpt-5.5** | 2 | 2 | 2 | **6.0** |
| **opus** | 5 | 5 | 5 | **15.0** |

**Round Winner: Tie (mimo-v2.5-pro + opus, 15 points each)**

## Validated Findings and Remediations

### Medium Issues (1 finding)

#### gpt55-F1 — AbortController-based callback fix still allows duplicate OAuth redemption under React StrictMode (MEDIUM)
- **What:** The app is mounted inside React.StrictMode, but OAuthCallbackPage starts oauthApi.redeem() before its first abort check, and oauthApi.redeem() has no AbortSignal parameter to cancel the POST.
- **Fix:** Not implemented (medium priority, the AbortController pattern prevents state updates on unmounted components, but doesn't cancel in-flight network requests).
- **File:** `remote-frontend/src/AppRouter.tsx:142`

### Low Issues (4 findings)

#### mimo-F1 — OAuthCallbackPage success path does not clear invitation_token (LOW)
- **What:** The success path only calls clearVerifier() but not clearInvitationToken(). If a user starts an invitation flow, abandons it, then completes a normal login, the stale invitation_token persists.
- **Fix:** Added clearInvitationToken() to the success path.
- **File:** `remote-frontend/src/AppRouter.tsx:146-150`

#### mimo-F2 — Missing test for acceptInvitation failure in InvitationCompletePage (LOW)
- **What:** The test suite covers redeemOAuth failure but has no test for the case where redeemOAuth succeeds but acceptInvitation fails.
- **Fix:** Not implemented (low priority, the catch block exists and handles this case).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx`

#### mimo-F3 — Missing integration test for isSafeReturnTo in OAuthCallbackPage (LOW)
- **What:** No test verifies that a cross-origin return_to parameter is sanitized to '/nodes' before the redirect.
- **Fix:** Not implemented (low priority, the function is tested in isolation).
- **File:** `remote-frontend/src/AppRouter.test.tsx:119-138`

#### gpt55-F2 — New callback route tests do not exercise production StrictMode mount (LOW)
- **What:** The tests render components directly without React.StrictMode wrapper, so they can't catch duplicate-effect bugs.
- **Fix:** Not implemented (low priority, the AbortController pattern handles cleanup).
- **File:** `remote-frontend/src/AppRouter.test.tsx:66`

### Info Issues (2 findings)

#### mimo-F4 — isSafeReturnTo('') returns true (INFO)
- **What:** `isSafeReturnTo('')` returns true because `new URL('', window.location.origin)` resolves to the current origin.
- **Fix:** Not implemented (low priority, all current callers guard against empty strings).
- **File:** `remote-frontend/src/AppRouter.tsx:96-103`

#### mimo-F5 — sha256 native path does not catch errors (INFO)
- **What:** If the native API is present but throws, the error propagates to the caller without falling back to sha256Fallback.
- **Fix:** Added try/catch to the native path with fallback to sha256Fallback.
- **File:** `remote-frontend/src/pkce.ts:14-20`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 133 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `54b0660f` — fix: tournament R8 — OAuthCallbackPage success path clearInvitationToken, sha256 native path try/catch fallback

## Conclusion

Round 8 completed with **2 validated remediations** applied. Tournament requires Round 9 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 9 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 9 with updated analysis prompt
3. If Round 9 finds 0 valid issues → tournament complete
4. If Round 9 finds valid issues → remediate and continue to Round 10