# Tournament Round 14 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: mimo-v2.5-pro, deepseek-v4-pro, glm-5.2

## Validated Remediations

### Medium (1)
- **mimo-F1**: `acceptInvitation()` raw fetch → routed through `makeRequest` with AbortSignal + 30s timeout

### Low (3)
- **mimo-F2/deepseek-F3/glm-F2**: `getInvitation()` raw fetch → routed through `makeRequest` with AbortSignal
- **mimo-F3**: `oauthApi.init()` missing signal parameter → added optional `signal` param
- **glm-F3**: InvitationCompletePage catch block ordering → abort check before storage cleanup

## Commit: `7aab2e38`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
