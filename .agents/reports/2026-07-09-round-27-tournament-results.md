# Tournament Round 27 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: opus, glm-5.2, mimo-v2.5-pro

## Validated Remediations

### Low (2)
- **opus-F1/mimo-F1**: Fixed VITE_API_BASE_URL trailing slash producing double-slash request URLs
- **mimo-F2**: Added loading guard for double-click prevention on OAuth login buttons

## Commit: `693fc2a0`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 137 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
