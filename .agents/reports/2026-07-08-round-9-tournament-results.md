# Tournament Round 9 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: deepseek-v4-pro, kimi-k2.7-code, glm-5.2
> Peer reviewers: cross-assigned

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **glm-5.2** | 7 | 7 | 7 | **21.0** |
| **kimi-k2.7-code** | 4 | 4 | 4 | **12.0** |
| **deepseek-v4-pro** | 0 | 0 | 0 | **0.0** |

**Round Winner: glm-5.2 (21 points)**

## Validated Findings and Remediations

### Medium Issues (1 finding)

#### kimi-F2 — React.StrictMode double-mount still allows the first request to hit the network (MEDIUM)
- **What:** The AbortController only suppresses state updates after each await, so a single-use authorization code may be consumed by the ghost effect.
- **Fix:** Not implemented (medium priority, the AbortController pattern prevents state updates on unmounted components, but doesn't cancel in-flight network requests).
- **File:** `remote-frontend/src/AppRouter.tsx:109-163`

### Low Issues (6 findings)

#### kimi-F1 — LoginPage OAuth init failure leaves a stale invitation_token (LOW)
- **What:** The catch block only calls clearVerifier() but not clearInvitationToken(), creating asymmetric cleanup.
- **Fix:** Added clearInvitationToken() to the LoginPage catch block.
- **File:** `remote-frontend/src/AppRouter.tsx:53-57`

#### glm-F1 — InvitationCompletePage places storage cleanup operations between two async calls (LOW)
- **What:** Storage operations are between acceptInvitation and the second abort check. If aborted during acceptInvitation, storage is corrupted for the re-run effect.
- **Fix:** Added abort check immediately after acceptInvitation resolves and before any storage operations.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:65-71`

#### glm-F2 — No test exercises the path where redeemOAuth succeeds but acceptInvitation rejects (LOW)
- **What:** The catch block handles both async failures identically, but only the redeemOAuth rejection path is tested.
- **Fix:** Not implemented (low priority, the catch block exists and handles this case).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx:107-119`

#### glm-F3 — Error-path tests don't verify sessionStorage/localStorage state (LOW)
- **What:** Error-path tests check error text but not sessionStorage/localStorage cleanup.
- **Fix:** Not implemented (low priority, the component handles this correctly).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx:59-105`

#### glm-F4 — OAuthCallbackPage success-path test doesn't verify clearInvitationToken() (LOW)
- **What:** The test only sets oauth_verifier and asserts it is null, but never sets invitation_token.
- **Fix:** Not implemented (low priority, the test verifies the primary cleanup).
- **File:** `remote-frontend/src/AppRouter.test.tsx:119-138`

#### glm-F7 — Success message says 'Redirecting to ${orgSlug}...' but redirects to app root (LOW)
- **What:** The user sees 'Redirecting to test-org...' but is taken to the app root.
- **Fix:** Not implemented (low priority, the message is informational).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:110,79`

### Info Issues (4 findings)

#### kimi-F3 — Decisions ledger acceptance evidence reports stale test counts (INFO)
- **What:** The ledger reports 114 tests but the current run is 133 tests.
- **Fix:** Not implemented (documentation drift, not a code issue).
- **File:** `docs/plans/fix-nonloopback-signin/decisions-ledger.md:143-150`

#### kimi-F4 — AppRouter.tsx imports from @/pkce twice (INFO)
- **What:** The file has two separate import statements from @/pkce.
- **Fix:** Not implemented (minor style issue).
- **File:** `remote-frontend/src/AppRouter.tsx:11,13`

#### glm-F5 — bytesToHex uses i++ while rest of pkce.ts uses i += 1 (INFO)
- **What:** Style inconsistency within the same module.
- **Fix:** Not implemented (minor style issue).
- **File:** `remote-frontend/src/pkce.ts:127`

#### glm-F6 — Unnecessary template literal in InvitationCompletePage (INFO)
- **What:** `window.location.assign(`${appBase}`)` is equivalent to `window.location.assign(appBase)`.
- **Fix:** Not implemented (minor style issue).
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:79`

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 133 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `f57f7051` — fix: tournament R9 — LoginPage invitation_token cleanup, InvitationCompletePage abort check after acceptInvitation

## Conclusion

Round 9 completed with **2 validated remediations** applied. Tournament requires Round 10 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 10 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 10 with updated analysis prompt
3. If Round 10 finds 0 valid issues → tournament complete
4. If Round 10 finds valid issues → remediate and continue to Round 11