# Tournament Round 5 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: deepseek-v4-pro, gpt-5.5, glm-5.2
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **glm-5.2** | 2 | 2 | 2 | **6.0** |
| **deepseek-v4-pro** | 3 | 3 | 3 | **9.0** |
| **gpt-5.5** | 1 | 1 | 1 | **3.0** |

**Round Winner: deepseek-v4-pro (9 points)**

## Validated Findings and Remediations

### Critical/Medium Issues (3 findings)

#### gpt55-F1 / glm-F1 — StrictMode interaction bug (MEDIUM)
- **What:** The `active` flag is set to `false` by the cleanup function, but the `hasRun` ref prevents the second effect from running. This means that when the async operation completes, it checks `if (!active) return` and skips the storage operations.
- **Fix:** Moved storage operations (clearVerifier, clearInvitationToken, localStorage.setItem) BEFORE the `if (!active)` guard so they execute regardless of mount state. The `active` flag now only gates React state setters (setSuccess, setOrgSlug, setError) and the timer.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:70-88`

#### deepseek-F1 — InvitationPage fetch useEffect has no abort/cleanup guard (MEDIUM)
- **What:** The fetch useEffect has no cleanup guard, which can lead to state updates on unmounted components.
- **Fix:** Added `active` flag with cleanup function to prevent state updates on unmounted components.
- **File:** `remote-frontend/src/pages/InvitationPage.tsx:23-29`

#### deepseek-F3 — LoginPage stale error persists after in-app URL cleanup (MEDIUM)
- **What:** The useEffect only sets error when urlError is truthy, but never clears when urlError becomes null.
- **Fix:** Updated useEffect to always set error from searchParams, clearing it when the param is removed.
- **File:** `remote-frontend/src/AppRouter.tsx:34-39`

### Low Issues (2 findings)

#### deepseek-F2 — No test for acceptInvitation failure after redeemOAuth succeeds (LOW)
- **What:** The test suite only exercises the redeemOAuth rejection path and success path, leaving a gap in error path coverage.
- **Fix:** Not implemented (low priority, the catch block exists and handles this case).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx`

#### glm-F2 — InvitationPage fetch catch uses unguarded `e.message` (LOW)
- **What:** The fetch catch uses `e.message` without an `instanceof Error` guard, unlike every other catch in the workstream.
- **Fix:** Added `instanceof Error` guard with fallback message.
- **File:** `remote-frontend/src/pages/InvitationPage.tsx:28`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 133 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `af700dca` — fix: tournament R5 — StrictMode interaction bug, InvitationPage fetch cleanup, error handling guard

## Conclusion

Round 5 completed with **3 validated remediations** applied. Tournament requires Round 6 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 6 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 6 with updated analysis prompt
3. If Round 6 finds 0 valid issues → tournament complete
4. If Round 6 finds valid issues → remediate and continue to Round 7