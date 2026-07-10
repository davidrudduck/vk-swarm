# Adversarial review — fix-nonloopback-signin (round 1, GLM)

**Reviewer:** GLM-5.2 (openrouter/z-ai/glm-5.2)
**Target:** `fix-nonloopback-signin` workstream diff (`main..HEAD`, 42 files)
**Date:** 2026-07-08
**Scope:** implementation fidelity, real bugs, cross-task interaction, test honesty

## Verification performed (gates re-run, not trusted from ledger)

| Check | Command | Result |
|---|---|---|
| Targeted tests | `npx vitest run src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx` | PASS 4 files / 12 tests |
| Full FE suite | `npx vitest run` | PASS 25 files / 114 tests |
| Lint | `npx eslint src --max-warnings 0` | exit 0 |
| Typecheck | `npx tsc --noEmit` | exit 0 |

Diff scope confirmed: the workstream's production-code change is **only** `remote-frontend/src/pkce.ts` (+ `vite.config.ts` exclude) and **four new/edited test files**. `InvitationCompletePage.tsx`, `InvitationPage.tsx`, `AppRouter.tsx`, `api.ts`, `oauth.ts` are **unchanged** vs `main`.

## Cryptographic review of the fallback (the core change)

`remote-frontend/src/pkce.ts:46-104` `sha256Fallback` reviewed line-by-line:

- Padding: `paddedLength = ceil((n+9)/64)*64`, `0x80` sentinel, 64-bit big-endian length (`view.setUint32(paddedLength-8, hi)`, `setUint32(paddedLength-4, lo >>> 0)`) — correct for all `n`, including the `n=55` boundary and the empty-input case.
- Message schedule: `s0/s1` small-sigma use `rotr(7)^rotr(18)^(x>>>3)` and `rotr(17)^rotr(19)^(x>>>10)` — correct.
- Compression: `ch`, `maj`, big-sigma `s0/s1`, `t1/t2`, state rotation, and the final `>>> 0` on every accumulator — correct.
- `rotateRight(v,b) = (v>>>b)|(v<<(32-b))`; every consumer ends in `>>> 0`, so the signed-Int32 intermediates are safely coerced back to unsigned. Correct.
- Verified against known vectors asserted by `pkce.test.ts`: `'' → e3b0c442…b855` and `'abc' → ba7816bf…15ad`. The empty-string vector is a strong test because it exercises the padding/length path with zero data bytes.

**Verdict on the digest: cryptographically correct. No finding.**

## Findings

### F1 — MEDIUM — `useEffect` cleanup leak in `InvitationCompletePage.tsx`, newly papered over by a green test

**Location:** `remote-frontend/src/pages/InvitationCompletePage.tsx:23-75` (the `useEffect` at `:23`, the swallowed cleanup at `:66`, the fire-and-forget call at `:74`); exposed by the workstream's new `remote-frontend/src/pages/InvitationCompletePage.test.tsx`.

**Mechanics:**
The `useEffect` callback is a *block-bodied* arrow function with **no `return` statement**:

```ts
useEffect(() => {
  const completeInvitation = async () => {
    ...
    const timer = setTimeout(() => {
      window.location.assign(`${appBase}`)   // :62-65
    }, 2000)
    return () => clearTimeout(timer)         // :66 — resolves the Promise, NOT returned to React
  }
  completeInvitation()                      // :74 — statement; return value (Promise) discarded
}, [handoffId, appCode, oauthError, urlToken])
```

React's effect contract: the callback must *return* the cleanup function (or `undefined`). Here the callback returns `undefined` (block body, no `return`); the `() => clearTimeout(timer)` returned by the async `completeInvitation` is the resolved value of a Promise that nobody awaits or returns. **React never registers a cleanup.** The 2000ms `setTimeout` that calls `window.location.assign(appBase)` is therefore **never cleared on unmount**.

**Production symptom:** after "Invitation accepted!", if the user navigates within the SPA during the 2-second window (back button, a link, a router push), the stale timer still fires `window.location.assign`, hijacking the user's navigation away from wherever they went.

**Why this is in-scope for THIS workstream, not just "pre-existing":**
- The workstream **added** `InvitationCompletePage.test.tsx` (SC7 acceptance evidence). That test renders the success path, schedules the real 2000ms timer, asserts "Invitation accepted!", and exits **well before 2000ms** — so it gives a green signal over a file containing a real bug. The test exercises neither the unmount-during-the-2s-window case nor the timer-cleanup contract. This is precisely the "false-green test that masks a real regression" class the testing-standards rule exists to prevent.
- `InvitationCompletePage.tsx` is inside the workstream's stated acceptance boundary (spec SC7: "Invitation completion still retrieves the stored invitation token with no storage-key changes").
- The workstream's rules (`AGENTS.md` "No Deferred Remediation" + "Pre-existing debt discovered during a session") require that a code-review finding from any review step be **fixed in-session**, **dismissed with ledger evidence**, or **escalated to the user** — never carried forward. This is not a false positive (the React effect-cleanup contract is unambiguous), so "fix next session" is not an available option.

**Recommendation (pick one, before the workstream PR is merged):**
1. **Fix in-session (preferred — small):** make the effect return a real cleanup that also guards state writes after unount:

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

2. **Split (legitimate scope split per AGENTS.md):** create `dev-docs/workstreams/fix-invitation-complete-timer-leak/README.md` **in this session** and add a decisions-ledger entry here documenting: pre-existing bug, out of workstream diff, not a red gate, split with a tracked follow-up. This is the sanctioned path if you judge the fix out of this workstream's surgical scope.

3. **Escalate to the user** if you believe the fix is architecturally entangled (it is not — it's a localized effect-cleanup fix).

**Effort:** fix ~15 min (code + extend the test to assert the timer is cleared on early unmount); split ~10 min (workstream README + ledger entry).

**Note on test flakiness:** I could not reproduce a visible failure today — jsdom environment teardown clears the pending timer before it fires in the current run. The leak is nonetheless real at the production contract level and is a latent suite flakiness source if timing or ordering changes. The substantive harm is the production navigation-hijack, not the test noise.

---

### F2 — LOW — Spec deviation on `window.location.assign` restoration, justified but undocumented in-spec

**Location:** `remote-frontend/src/AppRouter.test.tsx:94-99,115` and `remote-frontend/src/pages/InvitationPage.test.tsx:54-59,76`.

The spec (line 265-266) requires all tests to "restore `globalThis.crypto`, `window.location.assign`, storage, and mocks after each case." The new route tests deliberately **never spy on `window.location.assign`** — they keep `initOAuth`'s mock promise perpetually pending (the `resolveInitOAuth` never called, asserted as `'function'` at `AppRouter.test.tsx:115`) so the production `window.location.assign(result.authorize_url)` line is never reached. This dodges jsdom's "Not implemented: navigation to another Document" brittleness. The ledger (line 79-81) documents this as an intentional trade-off, and the actual navigation proof is deferred to the manual Playwright LAN check (task 301).

**Assessment:** justified deviation. The manual LAN evidence (`provider.test/authorize?flow=login|invitation`) covers the real navigation that the route tests intentionally don't assert. Not actionable on its own — listed only so the spec-vs-implementation delta is explicit and a future reviewer doesn't flag the missing `assign` spy as an oversight. The one residual "Not implemented: navigation" warning in the suite comes from the *callback* test (`window.location.assign('/nodes')` at `AppRouter.tsx:135`), which is expected and harmless.

**Effort:** none required. Optionally add one line to the spec's test-strategy section noting the route tests intentionally stop at `initOAuth()`.

---

### F3 — LOW — Code smell: split string concatenation in `InvitationPage.test.tsx`

**Location:** `remote-frontend/src/pages/InvitationPage.test.tsx:26` (`'/invitations/invite-token' + '/accept'`) and `:28` (`'/invitations/:token' + '/accept'`).

The `+ '/accept'` split serves no runtime purpose and is mildly misleading (reads like dynamic construction where it's actually a static route literal). It produces the correct strings, so functionally fine. Likely an artifact of dodging a path-anchor extractor. Cosmetic only.

**Recommendation:** inline to `'/invitations/invite-token/accept'` and `'/invitations/:token/accept'`.

**Effort:** 2 min.

---

### F4 — LOW — `beforeEach` uses `clearAllMocks` (keeps implementations); stale `initOAuth` impl carries across tests

**Location:** `remote-frontend/src/AppRouter.test.tsx:42` (`vi.clearAllMocks()`).

`vi.clearAllMocks()` resets call/results history but **not** implementations. The login test installs `initOAuth.mockReturnValue(<pending Promise>)` (`:95`); after `beforeEach`'s `clearAllMocks`, that pending-promise implementation persists into the callback test (`:118`), which doesn't invoke `initOAuth` so no harm occurs today. Latent smell: a future test that calls `initOAuth` without re-arming it would inherit the never-resolving promise and hang.

**Recommendation:** either (a) use `vi.resetAllMocks()` in `beforeEach` to drop implementations too, or (b) add a per-test `vi.mocked(initOAuth).mockReset()` where the pending-promise pattern is used. (a) is the cleaner one-line fix.

**Effort:** 2 min.

---

## Non-findings (explicitly checked, no issue)

- **`data as BufferSource` cast** (`pkce.ts:17`): legitimate workaround for the TS 5.7+ `Uint8Array<ArrayBufferLike>` ↔ `BufferSource` lib.dom change; `tsc --noEmit` exits 0 with it. Documented in ledger (line 90-96). Not a hidden type hole.
- **`globalThis.crypto` vs bare `crypto` inconsistency** between `sha256` and `generateVerifier`: intentional per spec ("Keep `generateVerifier()` unchanged"). `crypto.getRandomValues` is available in all contexts (spec line 117-118), so bare access is safe here. Not a bug.
- **`vite.config.ts` `scripts/**` exclude:** verified `remote-frontend/scripts/` contains only `no-push-invariant.test.mjs`, which uses `node:test` (not Vitest) and is documented to run via `node --test`. The exclude hides no legitimate Vitest tests; it repairs a real pre-existing suite-collection bug (ledger line 159-166). Not a finding.
- **Native-branch test (`pkce.test.ts:23-35`):** genuinely exercises the `globalThis.crypto?.subtle` path (asserts `subtleDigest` called once with `'SHA-256'` and `[97,98,99]`), not a mock-past-the-seam. Good test.
- **Route tests use the real `@/pkce`** (not mocked): the fallback SHA-256 genuinely runs against a `crypto` stubbed to drop `subtle`. The "real seam" claim in the ledger (line 122-124) holds.
- **`crypto`/storage restoration in `afterEach`:** `vi.unstubAllGlobals()` + `sessionStorage.clear()` are present in both `pkce.test.ts` and the two route-test files; `InvitationCompletePage.test.tsx` doesn't stub globals and clears storage. No cross-test contamination observed (114/114 green).
- **Manual LAN evidence using `provider.test`:** `provider.test` is RFC-2606-reserved. The bug was that `crypto.subtle.digest` threw *before* `initOAuth()` could be called; reaching any provider authorize URL proves the local fix. Acceptable for this workstream's scope.

## Summary

| ID | Severity | Type | Location | In workstream diff? | Effort |
|---|---|---|---|---|---|
| F1 | Medium | Bug (pre-existing, false-green test) | `InvitationCompletePage.tsx:23-75` + new `InvitationCompletePage.test.tsx` | Test yes / prod no | 15 min fix or 10 min split |
| F2 | Low | Spec deviation (justified) | `AppRouter.test.tsx`, `InvitationPage.test.tsx` | Yes | none |
| F3 | Low | Smell | `InvitationPage.test.tsx:26,28` | Yes | 2 min |
| F4 | Low | Smell | `AppRouter.test.tsx:42` | Yes | 2 min |

**Bottom line:** The core PKCE SHA-256 fallback is cryptographically correct and well-tested against known vectors; all four gates pass. The one substantive finding (F1) is a pre-existing React `useEffect` cleanup leak in `InvitationCompletePage.tsx` that the workstream's newly-added green test cannot catch — and per the workstream's own "No Deferred Remediation" rule it must be fixed in-session, split to a tracked follow-up created this session, or escalated to the user before the PR merges. The remaining items are cosmetic.
