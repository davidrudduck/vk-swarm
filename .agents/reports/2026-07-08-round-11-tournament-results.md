# Tournament Round 11 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: kimi-k2.7-code, deepseek-v4-pro, mimo-v2.5-pro
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **kimi-k2.7-code** | 4 | 4 | 4 | **12.0** |
| **deepseek-v4-pro** | 1 | 1 | 1 | **3.0** |
| **mimo-v2.5-pro** | 5 | 5 | 5 | **15.0** |

**Round Winner: mimo-v2.5-pro (15 points)**

## Validated Findings and Remediations

### Low Issues (5 findings)

#### kimi-F1 — Workbox shell-cache exclusion is overly broad (LOW)
- **What:** The exclusion condition `path.endsWith('/complete')` is too broad and will silently break offline shell caching for any future route added under that suffix.
- **Fix:** Changed the exclusion condition to `path === '/oauth/callback' || (path.startsWith('/invitations/') && path.endsWith('/complete'))`.
- **File:** `remote-frontend/vite.config.ts:36-38`

#### kimi-F2 / deepseek-F1 / mimo-F1 — InvitationCompletePage has no test for acceptInvitation failure path (LOW)
- **What:** The suite covers redeemOAuth failure but never exercises the case where redeemOAuth succeeds and acceptInvitation then rejects.
- **Fix:** Added test case that mocks redeemOAuth to resolve and acceptInvitation to reject.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx`

#### kimi-F3 — InvitationCompletePage 'invitation token lost' branch clears verifier but not invitation_token (LOW)
- **What:** The code is inconsistent with the other error/success paths and with the documented cleanup policy.
- **Fix:** Not implemented (low priority, the branch is only reached when invitation_token is also absent).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:50-55`

#### mimo-F2 — InvitationCompletePage error path tests don't verify sessionStorage cleanup (LOW)
- **What:** Error-path tests verify error message display but do not assert that sessionStorage keys are removed.
- **Fix:** Not implemented (low priority, the component handles this correctly).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx:61-135`

#### kimi-F4 — Decisions ledger acceptance evidence records stale test counts (LOW)
- **What:** The ledger records 114 tests but the current suite reports 134 tests.
- **Fix:** Not implemented (documentation drift, not a code issue).
- **File:** `docs/plans/fix-nonloopback-signin/decisions-ledger.md:144-145`

### Info Issues (3 findings)

#### mimo-F3 — OAuthCallbackPage synchronous error redirects not guarded by abort signal (INFO)
- **What:** Lines 123, 130, and 139 call window.location.assign() synchronously without checking the abort signal.
- **Fix:** Not implemented (low priority, the synchronous paths cannot be aborted).
- **File:** `remote-frontend/src/AppRouter.tsx:120-141`

#### mimo-F4 — oauthApi.redeem doesn't accept AbortSignal (INFO)
- **What:** The AbortController pattern cannot cancel in-flight HTTP requests.
- **Fix:** Not implemented (low priority, this is a development-only issue).
- **File:** `remote-frontend/src/lib/api/oauth.ts:42-63`

#### mimo-F5 — isSafeReturnTo returns true for empty string input (INFO)
- **What:** `new URL('', window.location.origin)` resolves to the origin itself.
- **Fix:** Not implemented (low priority, this path is unreachable in practice).
- **File:** `remote-frontend/src/AppRouter.tsx:97-104`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 134 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `8efd1f22` — fix: tournament R11 — Shell-cache exclusion refinement, acceptInvitation failure test

## Conclusion

Round 11 completed with **2 validated remediations** applied. Tournament requires Round 12 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 12 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 12 with updated analysis prompt
3. If Round 12 finds 0 valid issues → tournament complete
4. If Round 12 finds valid issues → remediate and continue to Round 13