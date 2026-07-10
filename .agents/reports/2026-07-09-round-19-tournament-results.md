# Tournament Round 19 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: minimax-m3, deepseek-v4-pro, glm-5.2

## Validated Remediations

### Low (1)
- **minimax-F1**: OAuth callback error-path tests now assert localStorage access_token is null

## Not Remediated (documented as known gaps, repeated from earlier rounds)
- **minimax-F2/glm-F1**: anySignal listener leak — infrastructure issue, not a code bug
- **minimax-F3**: InvitationCompletePage ignores return_to param — design decision
- **deepseek-F1/F2**: Test exercises unreachable defensive code, redundant mock setup — minor test quality issues
- **deepseek-F3/glm-F2**: makeRequest auto-attaches Bearer token on OAuth calls — behavior change, documented

## Commit: `adddb5f3`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
