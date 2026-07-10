# Tournament Round 22 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: glm-5.2, kimi-k2.7-code, opus

## Results

- **opus**: 0 findings — **CLEAN ROUND** ✅
- **glm-5.2**: 3 findings (low: refresh_token discarded, ProfileProvider 401 detection; info: transient accept failure)
- **kimi-k2.7-code**: 4 findings (low: ProfileProvider 401 detection, InvitationCompletePage incomplete cleanup, OAuthCallbackPage drops safeReturnTo; info: generateVerifier assumes crypto)

## Validated Remediations

### Low (2)
- **glm-F2/kimi-F1**: ProfileProvider 401 detection uses typed ApiError check instead of string matching
- **kimi-F2**: InvitationCompletePage error branches now have complete cleanup (clearVerifier + clearInvitationToken + localStorage.removeItem)

## Commit: `053cf42e`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 136 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
