# Tournament Round 18 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: kimi-k2.7-code, deepseek-v4-pro, minimax-m3

## Validated Remediations

### Medium (1)
- **minimax-F1/kimi-F3**: InvitationCompletePage orgSlug redirect mismatch → removed orgSlug state, simplified message to "Redirecting..."

### Low (3)
- **deepseek-F2/minimax-F2**: InvitationPage returnTo token not encodeURIComponent'd → added encodeURIComponent
- **minimax-F5**: Error-path tests missing localStorage assertions → added `localStorage.getItem('access_token')` assertions to all error-path tests

## Commit: `56b7bd81`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
