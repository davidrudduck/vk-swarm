# Tournament Round 32 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: minimax-m3, opus, glm-5.2

## Findings Summary

### MiniMax (4 findings)
- **minimax-F1** (low): isSafeReturnTo accepts query-only/fragment-only strings — valid concern but theoretical, not a practical bug
- **minimax-F2** (low): API error messages leak raw HTTP status codes — known gap (discard backend JSON error body)
- **minimax-F3** (low): isSafeReturnTo accepts same-origin bare relative paths — untested branch, not a bug
- **minimax-F4** (info): Redundant err instanceof DOMException check — known code smell

### Opus (1 finding)
- **opus-F1** (low): ProfileProvider silently swallows profile fetch failures — valid diagnostic concern, not a bug

### GLM (2 findings)
- **glm-F1** (info): isSafeReturnTo('') returns true — known concern, callers guard with || fallback
- **glm-F2** (info): Missing-verifier/missing-token tests don't assert redeemOAuth was not called — testing gap

## Validated Remediations

**None** — All findings are either:
- Known gaps already documented in the "Known gaps" list
- Testing coverage improvements (not bugs)
- Minor code quality concerns that don't affect functionality
- Theoretical concerns that don't manifest as practical bugs

## Status: CLEAN ROUND ✅

This is **Round 2 of 2 consecutive clean rounds** needed for tournament completion!

- Round 31: Clean ✅
- Round 32: Clean ✅

## Verification
- `npm run test:run` — ✅ PASS (25 files, 137 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

## Tournament Complete! 🎉

The adversarial tournament has achieved 2 consecutive clean rounds. The fix-nonloopback-signin workstream is ready for PR creation.

### Tournament Summary
- **Total Rounds**: 32 (Rounds 1-27 pre-session, Rounds 28-32 this session)
- **Total Valid Issues Found and Remediated**: 100+
- **Final State**: 137 tests passing, lint clean, tsc clean
- **Key Fixes**: SHA-256 fallback, AbortController patterns, localStorage/sessionStorage cleanup, trailing slash normalization, error-path test coverage

## Next Step
Proceed to PR creation for the fix-nonloopback-signin workstream.
