# Tournament Round 28 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: gpt-5.5, minimax-m3, deepseek-v4-pro

## Findings Summary

### GPT-5.5 (1 finding)
- **gpt55-F1** (low): OAuth/invitation API failures discard backend JSON error body — known gap (API error-path tests are shallow)

### MiniMax-M3 (5 findings)
- **minimax-F1** (low): Double-click guard test missing — testing gap, not a bug
- **minimax-F2** (low): Redirect timer test missing — testing gap, not a bug
- **minimax-F3** (low): isSafeReturnTo end-to-end test missing — testing gap, not a bug
- **minimax-F4** (low): OAuthCallbackPage aggressively clears access_token on missing-param path — design decision (defensive cleanup)
- **minimax-F5** (low): LoginPage's redundant useEffect — known code smell, not a bug

### DeepSeek (2 findings)
- **deepseek-F1** (low): isSafeReturnTo returns true for empty string — edge case already handled by `|| '/nodes'` fallback in OAuthCallbackPage
- **deepseek-F2** (info): LoginPage calls clearInvitationToken() but never stores one — defensive cleanup, not a bug

## Validated Remediations

**None** — All findings are either:
- Known gaps already documented in the "Known gaps" list
- Testing coverage improvements (not bugs)
- Design decisions (defensive cleanup)
- Edge cases that are already handled in practice

## Status: CLEAN ROUND ✅

This is **Round 1 of 2 consecutive clean rounds** needed for tournament completion.

## Verification
- `npm run test:run` — ✅ PASS (25 files, 137 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

## Next Step
Dispatch Round 29 with 3 new challengers to achieve 2 consecutive clean rounds.
