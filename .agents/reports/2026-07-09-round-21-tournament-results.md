# Tournament Round 21 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: gpt-5.5, opus, kimi-k2.7-code

## Validated Remediations

### Medium (2)
- **opus-F1/kimi-F1**: ProfileProvider hard-codes /v1/profile → replaced with profileApi.get() using VITE_API_BASE_URL
- **gpt55-F2/kimi-F2**: LoginPage init failure leaves stale localStorage.access_token → added localStorage.removeItem('access_token')

## Commit: `e28d3c98`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 136 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
