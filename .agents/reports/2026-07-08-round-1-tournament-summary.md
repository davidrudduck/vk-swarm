# Tournament Round 1 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: claude-opus-4-8, xiaomi/mimo-v2.5-pro, z-ai/glm-5.2
> Peer reviewer: z-ai/glm-5.2

## Scoring

| Challenger | Issues Found | Remediation Proposed | Peer-Validated | Total |
|---|---|---|---|---|
| **Opus** | 4 (1 medium, 3 low) | 1 (Issue 1) | 1 ✅ | **6** |
| **GLM** | 4 (1 medium, 3 low) | 1 (F1) | 1 ✅ | **6** |
| **MiMo** | 1 should-fix + 5 info | 1 (S-01) | 1 ✅ | **3** |

**Winner: Tie (Opus + GLM, 6 points each)**

## Issues Found and Remediated

### Issue 1 (Opus) — Orphaned `no-push-invariant` test — MEDIUM
- **What:** `scripts/no-push-invariant.test.mjs` excluded from Vitest but nothing runs it
- **Fix:** Added `"test:invariants": "node --test scripts/no-push-invariant.test.mjs"` to package.json
- **Peer review:** VALID (GLM confirmed script addition safe, invariant test passes)
- **Commit:** `2b319169`

### Issue 2 (MiMo S-01) — `base64UrlEncode` spread overflow — SHOULD-FIX
- **What:** `String.fromCharCode(...array)` can overflow V8 stack for arrays >64KB
- **Fix:** Replaced spread with loop-based character construction
- **Peer review:** VALID (GLM confirmed functionally equivalent for 32-byte verifiers)
- **Commit:** `2b319169`

### Issue 3 (GLM F1) — `useEffect` cleanup leak in InvitationCompletePage — MEDIUM
- **What:** `return () => clearTimeout(timer)` inside async function never reaches React; 2000ms timer never cleared on unmount
- **Fix:** Added `let active = true` guard, restructured effect to return real cleanup from outer scope
- **Peer review:** VALID (GLM confirmed bug accurately described and fix correct)
- **Commit:** `2b319169`

## Issues Found but Not Remediated (informational)

- Opus Issue 2: Multi-block SHA-256 path has no value-asserting test (low)
- Opus Issue 3: Inconsistent crypto guarding between `sha256` and `generateVerifier` (low)
- Opus Issue 4: Trivially-true `resolveInitOAuth` assertion (low)
- GLM F2: Spec deviation on `window.location.assign` restoration (low, justified)
- GLM F3: Split string concatenation in InvitationPage test (low, cosmetic)
- GLM F4: `clearAllMocks` keeps pending-promise impl across tests (low, latent)
- MiMo I-01 through I-05: Informational coverage observations

## Verification

- `npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx` — PASS 12/12
- `npm run test:invariants` — PASS 1/1
- `npx tsc --noEmit` — PASS

## Next

Round 2 required. Tournament incomplete (need 2 consecutive clean rounds).
