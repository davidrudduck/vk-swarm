# Tournament Round 16 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: opus, deepseek-v4-pro, glm-5.2

## Results

- **opus**: 0 findings — **CLEAN ROUND** ✅
- **deepseek-v4-pro**: 1 finding (low — `ProfileProvider` raw fetch outside workstream scope)
- **glm-5.2**: 4 findings (medium: refresh_token discarded; low: initOAuth missing signal, sessionStorage test gaps; info: redirect message mismatch)

## Validated Remediations

### Low (2)
- **glm-F2**: `initOAuth` call sites now pass AbortSignal
- **glm-F3**: Error-path tests now assert sessionStorage cleanup

## Commit: `5a47e4b9`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
