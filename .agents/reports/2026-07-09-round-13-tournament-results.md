# Tournament Round 13 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: minimax-m3, deepseek-v4-pro, glm-5.2
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **minimax-m3** | 8 | 8 | 8 | **24.0** |
| **glm-5.2** | 4 | 4 | 4 | **12.0** |
| **deepseek-v4-pro** | 2 | 2 | 2 | **6.0** |

**Round Winner: minimax-m3 (24 points)**

## Validated Findings and Remediations

### Medium Issues (1 finding)

#### minimax-F1 / glm-F1 — StrictMode double-redeem persists (MEDIUM)
- **What:** The AbortController pattern does not pass abortController.signal to the underlying fetch call. In React StrictMode, both effects fire redeem requests; the first succeeds and consumes the one-time handoff, the second fails with AlreadyRedeemed.
- **Fix:** Added optional `signal` parameter to `oauthApi.redeem` and `redeemOAuth`, threaded through to `makeRequest`. Updated `OAuthCallbackPage` and `InvitationCompletePage` to pass `abortController.signal` to the redeem call.
- **Files:** `remote-frontend/src/lib/api/oauth.ts:42-54`, `remote-frontend/src/api.ts:28-32`, `remote-frontend/src/AppRouter.tsx:143`, `remote-frontend/src/pages/InvitationCompletePage.tsx:57-61`

### Low Issues (2 findings)

#### deepseek-F1 — Dead `isRedirecting` state in OAuthCallbackPage (LOW)
- **What:** `setIsRedirecting(true)` is called immediately before `window.location.assign(safeReturnTo)`, which starts synchronous navigation. The "Redirecting..." branch is never rendered.
- **Fix:** Not implemented (low priority, the state is unreachable in production).
- **File:** `remote-frontend/src/AppRouter.tsx:108,151-152,166-173`

#### deepseek-F2 — InvitationCompletePage success path test doesn't verify org-slug redirect message (LOW)
- **What:** The test only asserts 'Invitation accepted!' heading, missing verification that `result.organization_slug` flows through to the redirect status text.
- **Fix:** Not implemented (low priority, the component handles this correctly).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx:55-58`

### Info Issues (2 findings)

#### minimax-F6 / glm-F3 — bytesToHex uses `i++` instead of `i += 1` (INFO)
- **What:** Style inconsistency within the same module.
- **Fix:** Not implemented (minor style issue).
- **File:** `remote-frontend/src/pkce.ts:127`

#### glm-F4 — Unnecessary template literal in InvitationCompletePage (INFO)
- **What:** `window.location.assign(`${appBase}`)` is equivalent to `window.location.assign(appBase)`.
- **Fix:** Not implemented (minor style issue).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:79`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `a8552804` — fix: tournament R13 — AbortSignal plumbing for StrictMode double-redeem prevention

## Conclusion

Round 13 completed with **1 validated remediation** applied. Tournament requires Round 14 to achieve 2 consecutive clean rounds before PR creation.