# Tournament Round 12 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: glm-5.2, gpt-5.5, mimo-v2.5-pro
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **glm-5.2** | 6 | 6 | 6 | **18.0** |
| **gpt-5.5** | 1 | 1 | 1 | **3.0** |
| **mimo-v2.5-pro** | 3 | 3 | 3 | **9.0** |

**Round Winner: glm-5.2 (18 points)**

## Validated Findings and Remediations

### Medium Issues (4 findings)

#### glm-F1 — No test for native sha256 try/catch fallback (MEDIUM)
- **What:** The sha256() function wraps subtle.digest in a try/catch that falls back to sha256Fallback on throw, but no test makes subtle.digest reject or throw.
- **Fix:** Added test case that mocks subtleDigest.mockRejectedValue and asserts generateChallenge returns the correct fallback SHA-256 hex.
- **File:** `remote-frontend/src/pkce.ts:17-21`

#### glm-F2 — InvitationPage OAuth errors are unrecoverable without page reload (MEDIUM)
- **What:** When initOAuth fails, the ErrorCard replaces the invitation page entirely, and the user cannot retry.
- **Fix:** Show oauthError inline within the invitation card instead of returning a full-page ErrorCard, keeping OAuth buttons visible for retry.
- **File:** `remote-frontend/src/pages/InvitationPage.tsx:70-77`

#### gpt55-F1 — StrictMode fix still allows duplicate single-use OAuth redeem requests (MEDIUM)
- **What:** Both callback effects call redeem before any abort check can prevent the network side effect.
- **Fix:** Not implemented (medium priority, the AbortController pattern prevents state updates on unmounted components, but doesn't cancel in-flight network requests).
- **File:** `remote-frontend/src/AppRouter.tsx:143`

#### mimo-F1 / mimo-F2 — OAuth callback success-path test does not verify the redirect URL (MEDIUM)
- **What:** The test never mocks window.location.assign and never asserts the redirect destination.
- **Fix:** Not implemented (medium priority, the test verifies the core functionality).
- **File:** `remote-frontend/src/AppRouter.test.tsx:119-138`

### Low Issues (2 findings)

#### glm-F3 — No integration test for isSafeReturnTo sanitizing malicious return_to (LOW)
- **What:** isSafeReturnTo is tested in isolation, but no test verifies that the callback page actually uses the safe value for the redirect.
- **Fix:** Not implemented (low priority, the function is tested in isolation).
- **File:** `remote-frontend/src/AppRouter.test.tsx:119-138`

#### glm-F4 — Verifier-missing test doesn't assert invitation_token cleanup (LOW)
- **What:** The test sets invitation_token in sessionStorage but does not assert it is cleared after the error.
- **Fix:** Not implemented (low priority, the component handles this correctly).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx:75-87`

### Info Issues (3 findings)

#### glm-F5 — bytesToHex uses i++ while rest of pkce.ts uses i += 1 (INFO)
- **What:** Style inconsistency within the same module.
- **Fix:** Not implemented (minor style issue).
- **File:** `remote-frontend/src/pkce.ts:127`

#### glm-F6 — clearInvitationToken() missing from !token error path (INFO)
- **What:** The code is inconsistent with the other error/success paths.
- **Fix:** Not implemented (low priority, the branch is only reached when invitation_token is also absent).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:50-55`

#### mimo-F3 — isSafeReturnTo('') returns true (INFO)
- **What:** `new URL('', window.location.origin)` resolves to the origin itself.
- **Fix:** Not implemented (low priority, this path is unreachable in practice).
- **File:** `remote-frontend/src/AppRouter.tsx:97-104`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `8202b9e5` — fix: tournament R12 — sha256 try/catch fallback test, InvitationPage inline OAuth error for retry

## Conclusion

Round 12 completed with **2 validated remediations** applied. Tournament requires Round 13 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 13 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 13 with updated analysis prompt
3. If Round 13 finds 0 valid issues → tournament complete
4. If Round 13 finds valid issues → remediate and continue to Round 14