# Tournament Round 1 — fix-nonloopback-signin

> Date: 2026-07-08
> Challengers: claude-opus-4-8, xiaomi/mimo-v2.5-pro, z-ai/glm-5.2
> Peer reviewer: z-ai/glm-5.2
> Tournament directory: `docs/plans/fix-nonloopback-signin/reviews/tournament-round-1/`

## Scoring Summary

| Challenger | Issues Found | Remediation Proposed | Peer-Validated | Total Score |
|---|---|---|---|---|
| **Opus** | 4 (1 medium, 3 low) | 1 (Issue 1) | 1 ✅ | **6** |
| **GLM** | 4 (1 medium, 3 low) | 1 (F1) | 1 ✅ | **6** |
| **MiMo** | 1 should-fix + 5 info | 1 (S-01) | 1 ✅ | **3** |

**Round Winner: Tie (Opus + GLM, 6 points each)**

## Issues Found and Remediated

### Issue 1 (Opus) — Orphaned `no-push-invariant` test — MEDIUM
- **What:** `scripts/no-push-invariant.test.mjs` excluded from Vitest but nothing runs it
- **Fix:** Added `"test:invariants": "node --test scripts/no-push-invariant.test.mjs"` to package.json
- **Peer review:** VALID (GLM confirmed script addition safe, invariant test passes)
- **Commit:** `2b319169`
- **File:** `remote-frontend/package.json`

### Issue 2 (MiMo S-01) — `base64UrlEncode` spread overflow — SHOULD-FIX
- **What:** `String.fromCharCode(...array)` can overflow V8 stack for arrays >64KB
- **Fix:** Replaced spread with loop-based character construction
- **Peer review:** VALID (GLM confirmed functionally equivalent for 32-byte verifiers)
- **Commit:** `2b319169`
- **File:** `remote-frontend/src/pkce.ts:110-118`

### Issue 3 (GLM F1) — `useEffect` cleanup leak in InvitationCompletePage — MEDIUM
- **What:** `return () => clearTimeout(timer)` inside async function never reaches React; 2000ms timer never cleared on unmount
- **Fix:** Added `let active = true` guard, restructured effect to return real cleanup from outer scope
- **Peer review:** VALID (GLM confirmed bug accurately described and fix correct)
- **Commit:** `2b319169`
- **File:** `remote-frontend/src/pages/InvitationCompletePage.tsx:23-75`

## Issues Found but Not Remediated (Informational)

### Opus Issues (Not Remediated)
- **Issue 2:** Multi-block SHA-256 path has no value-asserting test (low)
- **Issue 3:** Inconsistent crypto guarding between `sha256` and `generateVerifier` (low)
- **Issue 4:** Trivially-true `resolveInitOAuth` assertion (low)

### GLM Issues (Not Remediated)
- **F2:** Spec deviation on `window.location.assign` restoration (low, justified)
- **F3:** Split string concatenation in InvitationPage test (low, cosmetic)
- **F4:** `clearAllMocks` keeps pending-promise impl across tests (low, latent)

### MiMo Issues (Not Remediated)
- **I-01 through I-05:** Informational coverage observations

## Verification Results

### Automated Gates
- `npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx` — ✅ PASS (12/12 tests)
- `npm run test:invariants` — ✅ PASS (1/1 tests)
- `npx tsc --noEmit` — ✅ PASS (0 diagnostics)
- `npm run lint` — ✅ PASS

### Manual Verification
- LAN OAuth test on `http://10.69.96.233:3002` — ✅ PASS
- Both `/login` and `/invitations/:token/accept` routes working correctly
- `crypto.subtle` absent on non-secure HTTP origins as expected

## Tournament Process Notes

### Challengers Performance
- **Opus:** Most thorough analysis, found4 issues including the orphaned test
- **GLM:** Strong peer review capabilities, found cleanup leak issue
- **MiMo:** Good at identifying edge cases, found spread overflow issue

### Remediation Quality
- All3 remediations were peer-validated
- All fixes were minimal and targeted
- No regressions introduced

### Peer Review Process
- GLM served as peer reviewer for all3 findings
- All findings required neutral decider confirmation
- No disputes or rejections occurred

## Conclusion

Round 1 completed successfully with3 validated remediations. Tournament requires Round 2 to achieve2 consecutive clean rounds before PR creation.

## Next Steps

1. Select3 random challengers for Round 2 from pool: gpt-5.5, deepseek-v4-pro, minimax-m3, kimi-k2.7-code
2. Dispatch Round 2 with updated analysis prompt
3. If Round 2 finds 0 valid issues → tournament complete
4. If Round 2 finds valid issues → remediate and continue to Round 3