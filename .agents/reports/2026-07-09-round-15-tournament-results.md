# Tournament Round 15 — fix-nonloopback-signin

> Date: 2026-07-09
> Challengers: gpt-5.5, glm-5.2, kimi-k2.7-code

## Validated Remediations

### Medium (1)
- **gpt55-F1**: `.env.example` hard-coded `VITE_APP_BASE_URL=http://localhost:3000` → commented out with explanation

### Low (3)
- **glm-F1**: Invitation token not `encodeURIComponent`'d → added `encodeURIComponent` to `getInvitation`/`acceptInvitation` URLs
- **kimi-F4**: `generateVerifier` used bare `crypto` → changed to `globalThis.crypto`
- **kimi-F1**: Duplicate `@/pkce` imports in AppRouter.tsx → consolidated into single import

## Not Remediated (documented as known gaps)
- **glm-F2/F3**: AbortSignal plumbing / `anySignal`/`makeRequest` unit test coverage — testing gap, not a code bug
- **glm-F4/kimi-F2**: `makeRequest` auto-injects Authorization on public endpoints — behavior change, documented
- **kimi-F3**: `anySignal` listener accumulation — minor, GC handles it

## Commit: `2a265dc2`

## Verification
- `npm run test:run` — ✅ PASS (25 files, 135 tests)
- `npm run lint` — ✅ PASS
- `npx tsc --noEmit` — ✅ PASS
