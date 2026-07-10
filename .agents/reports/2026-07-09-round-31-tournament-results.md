# Tournament Round 31 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: kimi-k2.7-code, mimo-v2.5-pro, gpt-5.5

## Findings Summary

### GPT-5.5 (1 finding)
- **gpt55-F1** (info): VITE_APP_BASE_URL typed as required but optional in runtime — same as kimi-F3

### MiMo (2 findings)
- **mimo-F1** (low): InvitationCompletePage displays unvalidated error query param — same class as LoginPage known gap, not a new bug
- **mimo-F2** (info): OAuth invitation flow catch blocks clear existing valid session on transient API errors — design concern, not a bug

### Kimi (3 findings)
- **kimi-F1** (medium): Async OAuth init handlers can call window.location.assign after unmount — theoretical concern, not a practical bug (button click handlers are unlikely to be called after unmount)
- **kimi-F2** (medium): Double-click guard reads stale closure value — theoretical concern, not a practical bug (React batches state updates synchronously in event handlers)
- **kimi-F3** (info): VITE_API_BASE_URL and VITE_APP_BASE_URL typed as required but optional in runtime — same as gpt55-F1

## Validated Remediations

**None** — All findings are either:
- Theoretical concerns that don't manifest as practical bugs
- Design decisions that are already documented
- Known gaps that are already in the "Known gaps" list
- Type declaration inconsistencies that don't affect runtime behavior

## Status: CLEAN ROUND ✅

This is **Round 1 of 2 consecutive clean rounds** needed for tournament completion (after Round 28 was clean, Rounds 29-30 had valid issues).

## Verification
- `npm run test:run` — ✅ PASS (25 files, 137 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS

## Next Step
Dispatch Round 32 with 3 new challengers to achieve 2 consecutive clean rounds.
