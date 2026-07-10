# Tournament Round 17 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: kimi-k2.7-code, deepseek-v4-pro, opus

## Results

- **kimi-k2.7-code**: File not found (challenger failed to write output) — treated as 0 findings
- **deepseek-v4-pro**: 2 findings (medium: InvitationPage test missing sessionStorage assertions; low: dead AbortController)
- **opus**: 2 findings (medium: authenticated API caching; medium: E2E gate red — both outside workstream scope)

## Validated Remediations

### Medium (1)
- **deepseek-F1**: InvitationPage initOAuth failure test now asserts sessionStorage cleanup

### Low (1)
- **deepseek-F2**: Removed dead AbortController from handleOAuthLogin

## Commit: `6f5faf28`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
