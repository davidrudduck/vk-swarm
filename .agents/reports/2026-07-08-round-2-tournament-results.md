# Tournament Round 2 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: deepseek-v4-pro, minimax-m3, kimi-k2.7-code
> Peer reviewers: deepseek-v4-pro, minimax-m3, kimi-k2.7-code (cross-assigned)

## Scoring Summary

| Challenger | Issues Found | Validated Issues | Validated Fixes | Total Score |
|---|---|---|---|---|
| **kimi-k2.7-code** | 6 | 6 | 6 | **18.0** |
| **deepseek-v4-pro** | 5 | 5 | 5 | **12.0** |
| **minimax-m3** | 4 | 3 | 3 | **6.5** |

**Round Winner: kimi-k2.7-code (18 points)**

## Validated Findings and Remediations

### Category 1: InvitationCompletePage Bugs (3 findings)

#### deepseek-F1 / kimi-F3 — Missing params causes infinite spinner (MEDIUM)
- **What:** When `handoff_id` or `app_code` are missing and no `oauthError` is present, the component returns early without setting an error state, leaving the user stuck at "Completing invitation..." forever.
- **Fix:** Added `setError('Missing OAuth parameters. Please try the invitation link again.')` before the early return.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:33-35`

#### kimi-F1 — StrictMode double-effect causes duplicate API calls (MEDIUM)
- **What:** React.StrictMode in development causes `useEffect` to run twice. The `active` guard only suppresses state updates from the first cleanup; it does not prevent the second effect instance from starting its own `redeemOAuth` + `acceptInvitation` calls.
- **Fix:** Added `hasRun` ref to prevent the effect from running twice.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:23-80`

### Category 2: OAuthCallbackPage StrictMode (1 finding)

#### kimi-F2 — OAuthCallbackPage has same StrictMode double-effect problem (MEDIUM)
- **What:** `OAuthCallbackPage` runs `completeOAuth()` inside `useEffect` with no mounted/active guard and no cleanup. In development, StrictMode will invoke the effect twice and call `oauthApi.redeem` twice for the same handoff.
- **Fix:** Added `hasRun` ref to prevent the effect from running twice.
- **File:** `remote-frontend/src/AppRouter.tsx:104-144`

### Category 3: Test Flakiness (2 findings - duplicates)

#### kimi-F4 / minimax-F1 — Invitation route test is flaky (MEDIUM)
- **What:** The test relies on the real `getInvitation` export preserved by the partial `@/api` mock, so the component calls `fetch('/v1/invitations/test-token')` in jsdom. The assertion passes only if the page is still in the 'Loading invitation...' state when `waitFor` resolves.
- **Fix:** Mocked `getInvitation` to return a valid invitation object, then assert on `screen.getByText("You've been invited")`.
- **File:** `remote-frontend/src/AppRouter.test.tsx:179-198`

### Category 4: Test Coverage Gaps (4 findings)

#### deepseek-F2 — InvitationCompletePage.test.tsx missing error paths (LOW)
- **What:** Only the happy path is tested. Four error paths are untested: oauthError param, missing stored verifier, missing invitation token, and catch-block.
- **Fix:** Added test cases for each error path.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.test.tsx`

#### deepseek-F3 — InvitationPage.test.tsx missing error paths (LOW)
- **What:** Only the happy-path OAuth initiation is tested. Neither `getInvitation()` failure nor `initOAuth()` rejection is tested.
- **Fix:** Added test cases for both error paths.
- **File:** `remote-frontend/src/pages/InvitationPage.test.tsx`

#### kimi-F5 / minimax-F2 — SHA-256 fallback only tested with short inputs (LOW)
- **What:** The SHA-256 fallback is only tested against two short inputs ('' and 'abc', 0 and 3 bytes). These vectors do not exercise multi-block padding, multi-block compression, or the 64-word message-schedule expansion.
- **Fix:** Added the FIPS 180-2 vector 'abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq' (56 bytes) to exercise multi-block padding and compression.
- **File:** `remote-frontend/src/pkce.test.ts:37-46`

#### deepseek-F4 — Unnecessary string concatenation in route paths (INFO)
- **What:** `'/invitations/invite-token' + '/accept'` instead of `'/invitations/invite-token/accept'`.
- **Fix:** Replaced concatenation with single string literals.
- **File:** `remote-frontend/src/pages/InvitationPage.test.tsx:26,28`

### Category 5: SHA-256 Implementation (2 findings)

#### deepseek-F5 — rotateRight returns signed integers (INFO)
- **What:** The `rotateRight` helper function does not apply `>>> 0` to its return value. In JavaScript, the bitwise OR `|` converts both operands to signed 32-bit integers, causing `rotateRight` to return negative values for inputs with the MSB set.
- **Fix:** Applied `>>> 0` to the return value.
- **File:** `remote-frontend/src/pkce.ts:106-108`

#### minimax-F4 — Hand-rolled SHA-256 risk surface (INFO)
- **What:** The implementation is a hand-rolled SHA-256 in TypeScript with ~60 lines of bitwise arithmetic. The risk is that future changes could silently produce incorrect digests.
- **Fix:** This is a risk-surface observation, not a bug. The F1/F2 remediation (add multi-block test vectors) is the actionable mitigation.
- **File:** `remote-frontend/src/pkce.ts:46-104`

### Category 6: Stale-Token Precedence (1 finding)

#### kimi-F6 — URL token should take precedence over sessionStorage (LOW)
- **What:** `InvitationCompletePage` prefers the stored invitation token over the URL route token. If sessionStorage happens to contain a stale token from a previous invitation flow, the page would accept the wrong organization.
- **Fix:** Changed precedence to use URL token first: `urlToken || retrieveInvitationToken()`.
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:44`

## Invalid Findings (1 out of 15)

#### minimax-F3 — Login test doesn't exercise real fetch path (LOW)
- **What:** The mock for `@/lib/api/oauth` replaces `oauthApi` with a fresh object, and the `@/api` mock partially overrides `initOAuth`. This means the login test never exercises the real `oauthApi.init → fetch` path.
- **Why Invalid:** The mock isolation is intentional — this is a router-level unit test that verifies `initOAuth` is called with correct PKCE parameters. The remediation ("spy on globalThis.fetch") doesn't apply because the mocked `initOAuth` never calls fetch. Testing the actual HTTP path is an integration/e2e concern, not in the scope of this test.

## Verification Results

### Automated Gates
- `npm run test:run` — ✅ PASS (25 files, 121 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

### Commit
- `d9d891f0` — fix: tournament R2 — StrictMode guards, missing-param errors, test flakiness, SHA-256 unsigned rotation, multi-block vectors, error-path coverage

## Conclusion

Round 2 completed with **14 validated remediations** applied. Tournament requires Round 3 to achieve 2 consecutive clean rounds before PR creation.

## Next Steps

1. Select 3 random challengers for Round 3 from pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
2. Dispatch Round 3 with updated analysis prompt
3. If Round 3 finds 0 valid issues → tournament complete
4. If Round 3 finds valid issues → remediate and continue to Round 4