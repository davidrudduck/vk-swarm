# Tournament Round 30 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: glm-5.2, minimax-m3, deepseek-v4-pro

## Findings Summary

### DeepSeek (0 findings)
- Clean pass — no issues found

### GLM (6 findings)
- **glm-F1** (medium): False-green test in InvitationCompletePage — "shows error when acceptInvitation fails after successful redeem" doesn't seed localStorage.access_token before asserting toBeNull() — **VALID** ✅
- **glm-F2** (medium): False-green test in InvitationCompletePage — "shows error when handoff_id and app_code are missing" doesn't seed localStorage.access_token before asserting toBeNull() — **VALID** ✅
- **glm-F3** (medium): False-green test in InvitationPage — "shows error when initOAuth fails" doesn't seed localStorage.access_token before asserting toBeNull() — **VALID** ✅
- **glm-F4** (medium): False-green test in InvitationPage — "shows error when getInvitation fails" doesn't assert sessionStorage/localStorage cleanup — FALSE POSITIVE (no OAuth flow started, no cleanup needed)
- **glm-F5** (low): LoginPage initOAuth fails test doesn't assert localStorage cleanup — **VALID** ✅
- **glm-F6** (info): InvitationPage's OAuthButton shows no loading text — minor UX concern

### MiniMax (6 findings)
- **minimax-F1** (low): InvitationCompletePage oauthError branch doesn't redirect — valid UX concern but not a bug
- **minimax-F2** (info): appBase calculation duplicated in 3 files — maintenance concern
- **minimax-F3** (info): Double-click guard pattern duplicated — maintenance concern
- **minimax-F4** (low): InvitationPage.test.tsx error-path test uses getAllByText which is brittle — testing concern
- **minimax-F5** (low): setIsRedirecting(true) + window.location.assign creates flash of "Redirecting..." — UX concern
- **minimax-F6** (low): ProfileProvider checks err.status instead of err.statusCode — code quality concern

## Validated Remediations

### Medium (3)
- **glm-F1**: Added localStorage.setItem('access_token', 'stale-token') to InvitationCompletePage "shows error when acceptInvitation fails after successful redeem" test
- **glm-F2**: Added localStorage.setItem('access_token', 'stale-token') to InvitationCompletePage "shows error when handoff_id and app_code are missing" test
- **glm-F3**: Added localStorage.setItem('access_token', 'stale-token') to InvitationPage "shows error when initOAuth fails" test

### Low (1)
- **glm-F5**: Added localStorage.setItem('access_token', 'stale-token') and assertion to LoginPage initOAuth fails test

## Commit: `PENDING`

## Status: NOT CLEAN — 4 valid issues found and remediated

Round 28 was clean. Round 29 found 1 valid issue. Round 30 found 4 valid issues. The consecutive clean round streak is reset.

## Verification
- `npm run test:run` — ✅ PASS (25 files, 137 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

## Next Step
Dispatch Round 31 with 3 new challengers to continue the tournament.
