# Round 1 — GLM Peer Review of Proposed Remediations

**Reviewer:** GLM-5.2 (peer verifier)
**Date:** 2026-07-08
**Scope:** Verify three remediations proposed by Round-1 challengers (Opus Issue 1, MiMo S-01, GLM F1) against the real code in the `fix-nonloopback-signin` worktree.

Each remediation was checked against the actual source, the relevant config, and (where applicable) executed. Findings below.

---

## Remediation 1 (Opus Issue 1): Add `test:invariants` script to `remote-frontend/package.json`

### Verification steps

1. **`remote-frontend/package.json` has no such script.** ✅ Confirmed. The `scripts` block (lines 6-15) contains only: `dev`, `build`, `preview`, `lint`, `test`, `test:run`, `test:e2e`, `test:e2e:ci`. No `test:invariants`.
2. **`no-push-invariant.test.mjs` uses `node:test`.** ✅ Confirmed. Line 1: `import { test } from 'node:test';`. The file is a Node test runner file, not a Vitest file.
3. **The test was excluded from Vitest and nothing runs it.** ✅ Confirmed. `remote-frontend/vite.config.ts:74` has `exclude: ['**/node_modules/**', '**/e2e/**', '**/dist/**', 'scripts/**']`, which correctly stops Vitest from collecting the `node:test` file — but with no `test:invariants` script and no CI workflow invoking `node --test`, the file is orphaned from all automated execution. This matches Opus's "silent-disable" framing.
4. **`node --test scripts/no-push-invariant.test.mjs` passes from `remote-frontend/`.** ✅ Confirmed by execution:
   ```
   ✔ no new push channels (WebSocket/EventSource/SSE) in the hive frontend source (5.140811ms)
   ℹ tests 1  ℹ pass 1  ℹ fail 0
   ```
5. **The proposed script string is safe and correct.** ✅ The proposed `"test:invariants": "node --test scripts/no-push-invariant.test.mjs"` is exactly the command verified to pass. Adding it is a non-invasive, additive change with no regression risk.

### Caveat

The script addition alone makes the test **invokable** via `npm run test:invariants` but does not by itself wire **automated** execution. Opus's own finding text recommends *also* invoking it from `remote-hive-build.yml` (or opening a tracked follow-up). The AGENTS.md mandatory gate runs `cargo test --workspace` and `cd frontend && npm run lint` / `npx tsc --noEmit` but does **not** run any frontend test script — so without CI wiring or a follow-up, the test is still not automated, merely discoverable. The script is the necessary precondition for either of those, and is correct on its own terms.

### Verdict

**REMEDIATION 1: VALID.** The script addition is safe, correct, and the command is verified to pass. Full closure of the "orphaned from automated execution" finding additionally requires CI wiring or a tracked follow-up workstream (per Opus's own recommendation); the script alone is necessary-but-not-sufficient for *automation*, but as a remediation action it is sound.

---

## Remediation 2 (MiMo S-01): Replace spread in `base64UrlEncode`

### Verification steps

1. **The spread exists at `remote-frontend/src/pkce.ts:111`.** ✅ Confirmed:
   ```ts
   function base64UrlEncode(array: Uint8Array): string {
     const base64 = btoa(String.fromCharCode(...array))
   ```
2. **The loop replacement is functionally equivalent for 32-byte inputs.** ✅ Confirmed. The proposed loop
   ```ts
   let binary = ''
   for (let i = 0; i < array.length; i++) {
     binary += String.fromCharCode(array[i])
   }
   return btoa(binary)...
   ```
   produces the same `binary` string as `String.fromCharCode(...array)` for any input, including 32-byte arrays. `String.fromCharCode` applied element-wise vs. spread-applied is identical for `Uint8Array` values (each element is a code unit 0-255). The subsequent `btoa` + `.replace` chain is unchanged.
3. **`base64UrlEncode` is only called with 32-byte PKCE verifiers.** ✅ Confirmed. Grep across the repo finds exactly two hits in `pkce.ts`: the definition (line 110) and the single caller at line 4:
   ```ts
   export function generateVerifier(): string {
     const array = new Uint8Array(32)          // fixed 32 bytes
     crypto.getRandomValues(array)
     return base64UrlEncode(array)
   }
   ```
   No other caller exists in `crates/`, `remote-frontend/`, or elsewhere. The function is module-private (no `export`), so there are no external callers either.

### Assessment

The fix is **safe and correct** — it is functionally equivalent for the actual (32-byte) inputs and more robust for hypothetical future large inputs. However, the **claimed bug is unreachable in the current codebase**: there are no large-array callers, and `base64UrlEncode` is not exported, so no future caller can reach it without first editing `pkce.ts`. MiMo's own report is honest about this — it labels the finding `[SHOULD-FIX]` (not a live bug), states "Impact: Zero in practice — PKCE verifiers are always 32 bytes," and records the workstream-impact column as "None (32-byte input)." The spread-overflow threshold on V8 is ~65,536 arguments; 32 is nowhere near it.

### Verdict

**REMEDIATION 2: VALID** as a safe, functionally-equivalent defensive code-quality improvement. The fix introduces no regression and is correct. The bug framing ("stack overflow for large arrays") is **theoretical, not live** — no current or reachable caller exceeds 32 bytes — which matches MiMo's own impact assessment. Applying it is fine (it hardens the function against future misuse at zero cost), but it should not be presented as fixing a live defect.

---

## Remediation 3 (GLM F1): Fix `useEffect` cleanup leak in `InvitationCompletePage.tsx`

### Verification steps

1. **The bug description is accurate.** ✅ Confirmed by reading `remote-frontend/src/pages/InvitationCompletePage.tsx:23-75`:
   ```tsx
   useEffect(() => {                              // :23
     const completeInvitation = async () => {     // :24
       ...
       const timer = setTimeout(() => {           // :61
         window.location.assign(`${appBase}`)
       }, 2000)
       return () => clearTimeout(timer)           // :66 — INSIDE the async fn
     }
     completeInvitation()                         // :74 — statement; returns Promise, discarded
   }, [handoffId, appCode, oauthError, urlToken])  // :75 — effect returns undefined
   ```
   The `useEffect` callback is a block-bodied arrow with **no `return` statement**. `completeInvitation()` at line 74 is an expression statement — its return value (a `Promise` that resolves to `() => clearTimeout(timer)`) is discarded. The effect therefore returns `undefined` to React. **React never registers a cleanup.** The `return () => clearTimeout(timer)` at line 66 is the resolved value of an unawaited Promise — dead code that never reaches React. The 2000ms `setTimeout` calling `window.location.assign(appBase)` is never cleared on unmount, which can hijack navigation if the user navigates within the 2s window.

2. **The proposed fix is correct.** ✅ The proposed restructure:
   ```ts
   useEffect(() => {
     let active = true
     let timer: ReturnType<typeof setTimeout> | undefined
     ;(async () => {
       if (oauthError) { if (active) setError(`OAuth error: ${oauthError}`); return }
       if (!handoffId || !appCode) return
       try {
         const verifier = retrieveVerifier()
         if (!verifier) { if (active) setError('OAuth session lost. Please try again.'); return }
         const token = retrieveInvitationToken() || urlToken
         if (!token) { if (active) setError('Invitation token lost. Please try again.'); return }
         const { access_token } = await redeemOAuth(handoffId, appCode, verifier)
         const result = await acceptInvitation(token, access_token)
         if (!active) return
         clearVerifier(); clearInvitationToken()
         setSuccess(true); setOrgSlug(result.organization_slug)
         timer = setTimeout(() => {
           const appBase = import.meta.env.VITE_APP_BASE_URL || window.location.origin
           window.location.assign(`${appBase}`)
         }, 2000)
       } catch (e) {
         if (!active) return
         setError(e instanceof Error ? e.message : 'Failed to complete invitation')
         clearVerifier(); clearInvitationToken()
       }
     })()
     return () => { active = false; if (timer) clearTimeout(timer) }
   }, [handoffId, appCode, oauthError, urlToken])
   ```
   - **Returns a real cleanup to React** (`return () => { active = false; if (timer) clearTimeout(timer) }`) — fixes the contract violation.
   - **Guards state writes after unmount** (`if (active) setError(...)` / `if (!active) return`) — eliminates the "setState on unmounted component" hazard.
   - **`if (timer)` guards `clearTimeout`** against the case where unmount happens before the awaits complete (timer still `undefined`); `clearTimeout(undefined)` is a no-op anyway, so this is defensive and safe.
   - The early synchronous returns (`oauthError`, `!handoffId`, `!appCode`, `!verifier`, `!token`) all occur before any `await`, so `active` is guaranteed `true` there — the `if (active)` guards are harmless no-ops on those paths and meaningful only after the two `await`s.

3. **Happy-path behavior is unchanged.** ✅ For the success path with the component staying mounted: `active` stays `true`, both awaits complete, storage is cleared, `setSuccess(true)` / `setOrgSlug(...)` fire, the 2000ms timer is scheduled, fires, and `window.location.assign(appBase)` navigates. Identical observable behavior to the original. The only behavioral deltas are (a) the now-functional unmount cleanup (the bug fix) and (b) guarded state writes after unmount (defensive; no-op on happy path).

4. **Scope check.** ✅ `git log` confirms `InvitationCompletePage.tsx` was last touched by `ec0747bf feat: share tasks (#1210)` and `git diff --stat main...HEAD` shows **zero changes** to it in this workstream. It is pre-existing code — but the workstream added `InvitationCompletePage.test.tsx` as SC7 acceptance evidence, and that test renders the success path and exits before the 2000ms timer fires, giving a green signal over the buggy file. Per AGENTS.md "Pre-existing debt discovered during a session," this must be fixed in-session, split to a tracked follow-up created this session, or escalated — not carried forward. The fix is small (~15 min) and localized, so "fix in-session" is the right path.

### Minor note on the unmount-during-async edge case

In the proposed fix, the `if (!active) return` after `acceptInvitation` resolves skips `clearVerifier()` / `clearInvitationToken()` if the component unmounts in the microtask window between the await resolving and the next sync line. This could leave a stale verifier/token in `sessionStorage` in that narrow window. This is a defensible trade-off (don't touch storage of a gone component) and is a strictly narrower deviation than the original (which set state on an unmounted component — an explicit React anti-pattern). A stricter variant would clear storage unconditionally (storage is not React state and clearing it is always safe), but the proposed fix is acceptable and does not invalidate the remediation.

### Verdict

**REMEDIATION 3: VALID.** The bug is real and accurately described (the line-66 cleanup is dead code; React never receives a cleanup function). The proposed fix correctly returns a real cleanup to React, guards state writes after unmount, and preserves happy-path behavior. The fix is localized and within the workstream's scope per AGENTS.md's "Pre-existing debt discovered during a session" rule.

---

## Summary Table

| # | Remediation | Verdict | Notes |
|---|---|---|---|
| 1 | Add `test:invariants` script (Opus Issue 1) | **VALID** | Script safe & verified to pass; full automation also needs CI wiring or a tracked follow-up (Opus's own caveat). |
| 2 | Replace spread in `base64UrlEncode` (MiMo S-01) | **VALID** | Safe, functionally-equivalent defensive improvement; claimed bug is theoretical (no reachable caller exceeds 32 bytes), matching MiMo's own impact note. |
| 3 | Fix `useEffect` cleanup leak (GLM F1) | **VALID** | Bug accurately described; fix correctly returns real cleanup, guards state writes, preserves happy path. In-scope per AGENTS.md pre-existing-debt rule. |

All three remediations are safe to apply. Remediation 3 is the only one that fixes a live (pre-existing, contract-level) defect; Remediation 1 closes a real automation-wiring gap (with the CI caveat); Remediation 2 is a sound defensive hardening of unreachable-in-practice code.
