# Tournament Round 25 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: deepseek-v4-pro, opus, mimo-v2.5-pro

## Validated Remediations

### Medium (2)
- **mimo-F1**: Fixed makeRequest timeout error message (DOMException with user-friendly reason)
- **deepseek-F1**: Added ProfileProvider test for non-401 ApiError responses (500)

## Commit: `29891b79`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 137 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
