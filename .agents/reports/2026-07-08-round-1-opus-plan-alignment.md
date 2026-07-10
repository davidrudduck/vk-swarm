# Plan Alignment Review — fix-nonloopback-signin

> Panelist: claude-opus-4-8
> Round: 1
> Date: 2026-07-08
> Scope: Implementation vs frozen spec + decomposed plan

## Executive Summary

The implementation is **fully aligned** with the frozen spec and decomposed plan. All 11
success criteria are satisfied, all four tasks touched exactly their intended files (with
one documented pre-existing-gate repair in `vite.config.ts`), all three divergences from
the literal Before→After prescriptions are recorded in the decisions ledger with sound
justification, and the reachability gate's call-path trace, real-seam tests, and
incident-symptom assertion all hold. No remediation was required.

## SC Coverage

| SC | Status | Evidence |
|----|--------|----------|
| SC1 — `generateChallenge()` correct SHA-256 hex on secure + non-secure origins | ✅ Satisfied | `remote-frontend/src/pkce.ts:7-20` (capability-detected `sha256`); tested `remote-frontend/src/pkce.test.ts:23-54` (native `abc`→`000f10ff`, fallback `''`→`e3b0c442…`, `abc`→`ba7816bf…`) |
| SC2 — existing localhost flow unchanged (no regression) | ✅ Satisfied | Native branch preserved `pkce.ts:16-17`; native-path test `pkce.test.ts:23-35`; full suite green (114 tests) confirmed by ledger + re-run |
| SC3 — sign-in completes over `http://<lan-ip>` | ✅ Satisfied | Route test `remote-frontend/src/AppRouter.test.tsx:88-116` drives real `/login` with `crypto.subtle` removed and asserts `initOAuth` gets a 64-hex challenge; manual LAN evidence in ledger `decisions-ledger.md:154` |
| SC4 — invitation acceptance completes over `http://<lan-ip>` | ✅ Satisfied | `remote-frontend/src/pages/InvitationPage.test.tsx:46-77`; manual LAN evidence `decisions-ledger.md:155` |
| SC5 — `oauthApi.init()` gets same 64-char lowercase hex from both entry points | ✅ Satisfied | `AppRouter.test.tsx:107-111` and `InvitationPage.test.tsx:67-71` both assert `expect.stringMatching(/^[0-9a-f]{64}$/)`; payload shape `lib/api/oauth.ts:25-29` (`app_challenge`) |
| SC6 — callback still retrieves stored verifier, no key change | ✅ Satisfied | `AppRouter.test.tsx:118-137` (redeem with `stored-verifier`, `oauth_verifier` cleared); keys unchanged `pkce.ts:126` (`VERIFIER_KEY='oauth_verifier'`) |
| SC7 — invitation completion still retrieves stored token, no key change | ✅ Satisfied | `remote-frontend/src/pages/InvitationCompletePage.test.tsx:34-56` (accept with `stored-token`); key unchanged `pkce.ts:127` (`TOKEN_KEY='invitation_token'`) |
| SC8 — unit tests cover both digest implementations | ✅ Satisfied | `pkce.test.ts:23-35` (native) + `pkce.test.ts:37-54` (fallback) |
| SC9 — unit tests cover both PKCE OAuth entry points | ✅ Satisfied | `/login` in `AppRouter.test.tsx:88-116`; invitation in `InvitationPage.test.tsx:46-77` |
| SC10 — full mandatory gate green, no deferred failures | ✅ Satisfied | Recorded PASS lines `decisions-ledger.md:141-150`; enforced by `verify-301-evidence.sh:14-29` |
| SC11 — manual LAN verification for both routes, else escalate | ✅ Satisfied | `decisions-ledger.md:152-155` records both `/login` and `/invitations/:token/accept` over `http://10.69.96.233:3002` reaching provider authorize URLs with `crypto.subtle` absent |

## Plan Adherence — Per Task

### Task 101 — Implement browser-safe SHA-256 fallback — **Conformant**
- Files touched (commit `c01093e8`): `remote-frontend/src/pkce.ts`, `remote-frontend/src/pkce.test.ts` (+ ledger). Exactly the two writable files in `files:`. Read-only siblings `api.ts`/`setupTests.ts` untouched. ✅
- Before→After edit applied verbatim except a documented `as BufferSource` cast at `pkce.ts:17` (see Divergences). Constants `SHA256_INITIAL`/`SHA256_K`, `sha256Fallback`, `rotateRight` match prescription byte-for-byte.
- Test file created exactly as prescribed (`pkce.test.ts` == "Failing test" block).
- STOP triggers respected: no storage-key/signature changes; known vectors match; no Node APIs / deps added; native branch is tested (`pkce.test.ts:23-35`).
- Ledger entry made for the undictated cast (`decisions-ledger.md:88-95`). ✅

### Task 201 — Cover non-loopback login + callback storage — **Conformant (documented divergence)**
- Files touched (commit `8e0216f3`): `remote-frontend/src/AppRouter.test.tsx` (+ ledger). Matches `files:`. No production code edited. ✅
- Prescribed imports (`afterEach`, `fireEvent`), helper `stubGetRandomValuesOnlyCrypto`, expanded `beforeEach`/`afterEach`, and both new tests applied.
- Divergence: `@/api` mock uses partial `importOriginal` spread instead of literal full-replacement (see Divergences) — documented `decisions-ledger.md:97-104`.
- STOP triggers respected: no `window.location.assign` spy, no `user-event` dependency, `initOAuth` kept pending, callback test needs no production change.

### Task 202 — Cover invitation OAuth + completion storage — **Conformant**
- Files touched (commit `530853af`): created `InvitationPage.test.tsx` and `InvitationCompletePage.test.tsx`. Matches create-only `allowed_change: create`. Read-only siblings `Nodes.test.tsx`/`HomePage.tsx` untouched. ✅
- Both files created exactly as prescribed. Button names (`Continue with GitHub`) match `InvitationPage.tsx:92`. Completion assertions match `InvitationCompletePage` behavior.
- STOP triggers respected: no production edits, challenge is 64-hex, invitation token stored before redirect (`InvitationPage.test.tsx:74`), completion accepts with the stored (not URL) token (`InvitationCompletePage.test.tsx:50-51`).

### Task 301 — Full gates + manual LAN verification — **Conformant with one needed extra edit**
- Files listed: `decisions-ledger.md` only. Commit `d5f73ce4` edits only the ledger. ✅
- BUT the acceptance work also required commit `b6214299` editing `remote-frontend/vite.config.ts` (unlisted) to clear a pre-existing full-suite gate failure (see Divergences). Documented `decisions-ledger.md:159-166`.
- Acceptance-evidence section appended matches the prescribed template; `verify-301-evidence.sh` guard (`:14-29`) enforces PASS on every line and rejects placeholder/FAIL/unavailable/inconclusive text.
- STOP triggers: no gate marked PASS without exit 0; LAN checks recorded as reached-authorize-URL.

## Divergences

### D1 — `remote-frontend/vite.config.ts` edited (not in any task `files:`) — **Needed**
- **What:** commit `b6214299` added `'scripts/**'` to the Vitest `exclude` array.
- **Why:** Task 301's mandatory full `npm run test:run` collected `remote-frontend/scripts/no-push-invariant.test.mjs` — a `node:test` file — as a Vitest suite and failed. This is pre-existing debt from the `vk-swarm-hive-ui` workstream surfaced by task 301's gate.
- **Assessment:** Needed and correct. Reverting the exclusion would re-break the mandatory full-suite gate. The exclusion is surgical — the only file under `scripts/` is the node-test file, so no legitimate Vitest test is hidden. Documented in decisions ledger under "pre-existing full-suite gate repair" (`decisions-ledger.md:159-166`). **No remediation warranted.**

### D2 — `pkce.ts:17` casts `data as BufferSource` — **Needed**
- **What:** native branch is `subtle.digest('SHA-256', data as BufferSource)`; task 101 "After" showed `subtle.digest('SHA-256', data)`.
- **Why:** repo DOM typings reject `Uint8Array<ArrayBufferLike>` as `BufferSource` under `npx tsc --noEmit`.
- **Assessment:** Needed for the typecheck gate; zero runtime/behavior change. Documented `decisions-ledger.md:88-95`. **No remediation required.**

### D3 — `AppRouter.test.tsx:20-23` partial `@/api` mock via `importOriginal` — **Needed**
- **What:** `vi.mock('@/api', async (importOriginal) => ({ ...(await importOriginal()), initOAuth: vi.fn() }))` instead of the literal `vi.mock('@/api', () => ({ initOAuth: vi.fn() }))`.
- **Why:** the literal full-replacement mock hid the `getInvitation` export used by the pre-existing `/invitations/:token/accept` route test, which then rendered React Router's error boundary.
- **Assessment:** Needed to keep a neighbouring pre-existing test green. Documented `decisions-ledger.md:97-104`. **No remediation required.**

## Remediations Applied

**None.** All three divergences (D1–D3) are needed, correct, non-regressing, and already
documented in the decisions ledger.

## Reachability Gate Assessment

Source: `decisions-ledger.md:106-135` (VERDICT: PASS).

### (a) Call-path trace — **Correct** (cosmetic line-number fix applied post-review)
The traced path is real and executes on every non-secure-origin call:
`/login` → `generateChallenge` → `pkce.ts` `sha256()` capability check → `sha256Fallback` →
`bytesToHex` → `initOAuth`/`oauthApi.init` → `POST /v1/oauth/web/init`. Verified against source.
Two cited line numbers were slightly off (cosmetic, not a gate failure) — corrected after review:
- Ledger said `AppRouter.tsx:38`; actual is line 39 (line 38 is `generateVerifier()`).
- Ledger said `oauthApi.init(...)` at `AppRouter.tsx:44`; actual `initOAuth(...)` call is line 47.

### (b) Real-seam test — **Confirmed**
Both cited tests drive real entry points, not mocks past the seam:
- `AppRouter.test.tsx` renders `createMemoryRouter(createRoutes())` — the real `LoginPage` and the real, un-mocked `pkce.ts`; only the downstream `initOAuth` network seam is mocked.
- `InvitationPage.test.tsx` renders the real `InvitationPage` with real `pkce.ts`; mocks only `../api`.

### (c) Incident-symptom assertion — **Confirmed**
Incident F-2026-07-06-02 maps directly: the old `crypto.subtle.digest('SHA-256', data)` would throw `Cannot read properties of undefined (reading 'digest')` before `initOAuth()`. Both route tests remove `crypto.subtle` and prove the flow now reaches `initOAuth` with a valid challenge.

## Assessment

- **Goals met:** SC1–SC11 all satisfied with cited file:line evidence.
- **Plan followed:** Each task modified exactly its listed writable files. Storage keys, callback semantics, route structure, OAuth payload shape, and backend behavior are all unchanged.
- **Divergences:** Three, all needed and documented. None introduce regressions.
- **Remediation:** None required. One cosmetic fix applied (reachability gate line numbers).
- **Reachability gate:** (a)/(b)/(c) all hold.

**Overall: PASS. Implementation is plan-aligned; no remediation applied because none was needed.**
