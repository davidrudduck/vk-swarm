---
doc_type: spec
status: shipped
workstream: fix-nonloopback-signin
change_kind: bugfix
discovered: 2026-07-06
source: session/2026-07-06
severity: high
---

# Fix sign-in on non-loopback HTTP origins

## Intent (what / why)

Sign-in is broken when the Hive UI is accessed over a non-loopback HTTP origin
(e.g. `http://192.168.x.x:5173` or `http://myhost:5173`). The PKCE challenge
generation in `remote-frontend/src/pkce.ts:10` calls `crypto.subtle.digest()`,
which is `undefined` in non-secure contexts. Browsers restrict `crypto.subtle`
to secure origins (HTTPS or `localhost`/`127.0.0.1`).

This blocks all sign-in flows on plain-HTTP LAN access — a common setup for
local Hive deployments where TLS is not configured.

## Expected outcome confirmed

The expected outcome is not merely "avoid the exception" or "make the helper
return something." The expected user-visible outcome is:

- A user can open the Hive UI from another device on the LAN using a plain HTTP
  non-loopback origin such as `http://192.168.x.x:3002`.
- From `/login`, clicking GitHub or Google starts the OAuth provider redirect
  successfully instead of failing locally with a `crypto.subtle` error.
- From `/invitations/:token/accept`, clicking GitHub or Google starts the
  invitation OAuth provider redirect successfully instead of failing locally
  with a `crypto.subtle` error.
- The verifier is still stored for the OAuth callback, the invitation token is
  still stored for invitation completion, and the backend still receives the
  same challenge format it already accepts.
- Secure-origin behavior (`localhost`, `127.0.0.1`, HTTPS) remains unchanged
  except for added test coverage proving it did not regress.
- The workstream is complete only when implementation, helper tests,
  entry-point tests, static checks, repository gates, and manual LAN checks are
  all satisfied. No sub-item is optional and no sub-item moves to a later
  workstream.

## Users / who is affected

- Any user accessing the Hive UI over a non-loopback HTTP URL (LAN IP, custom
  hostname without TLS).
- Blocks node onboarding and all authenticated flows in that context.

## User stories

- US1: As a LAN user accessing the Hive UI over non-loopback HTTP, I can start
  normal OAuth sign-in from `/login` without the browser failing locally because
  `crypto.subtle` is unavailable.
- US2: As an invited LAN user accessing the Hive UI over non-loopback HTTP, I
  can accept an invitation and start invitation OAuth without the browser
  failing locally because `crypto.subtle` is unavailable.
- US3: As an existing secure-origin user, I keep the current localhost/HTTPS
  PKCE behavior and backend challenge format with no regression.
- US4: As an executor implementing this bugfix, I have a complete acceptance
  boundary covering fallback behavior, both OAuth entry points, storage
  preservation, static checks, repository gates, and manual LAN verification.

## Success criteria

1. SC1: `generateChallenge()` produces a correct SHA-256 hex digest on both secure
   and non-secure origins. → US1, US2, US3:
2. SC2: The existing PKCE sign-in flow works unchanged on `localhost` (no
   regression). → US3:
3. SC3: Sign-in completes successfully when the UI is served over `http://<lan-ip>`.
   → US1:
4. SC4: Invitation acceptance completes successfully when the UI is served over
   `http://<lan-ip>`. → US2:
5. SC5: `oauthApi.init()` receives the same challenge format from both login entry
   points: a 64-character lowercase SHA-256 hex digest. → US1, US2, US3:
6. SC6: The OAuth callback path still retrieves the stored verifier and redeems the
   handoff with no storage-key changes. → US3:
7. SC7: Invitation completion still retrieves the stored invitation token with no
   storage-key changes. → US2, US3:
8. SC8: Unit tests cover both digest implementations: native `crypto.subtle` and the
   fallback. → US1, US2, US3:
9. SC9: Unit tests cover both OAuth entry points that use PKCE: `/login` in
   `AppRouter.tsx` and invitation acceptance in `InvitationPage.tsx`. → US1, US2:
10. SC10: The final implementation passes the full mandatory gate with no deferred
   failures: `cargo clippy --all --all-targets --all-features -- -D warnings`,
   `cargo test --workspace`, `cd frontend && npm run lint`, and
   `cd frontend && npx tsc --noEmit`. → US4:
11. SC11: Manual LAN verification is completed for both `/login` and
    `/invitations/:token/accept`; if it cannot be completed in the execution
    environment, the executor must stop and escalate rather than claim the
    workstream is complete. → US1, US2, US4:

## Constraints

- Must not weaken the PKCE security model — the fallback SHA-256 must be
  cryptographically correct.
- Should prefer native `crypto.subtle` when available (secure contexts) and
  only fall back on non-secure origins.
- Keep the fix in `pkce.ts` — no changes to the OAuth/PKCE protocol or backend.
- No item in this workstream is optional. The digest fallback, login coverage,
  invitation coverage, and validation gates are all required for completion.
- Do not mark tests ignored or create a follow-up workstream to finish the PKCE
  fallback later. This workstream finishes the bugfix end to end.
- Do not reduce scope to a helper-only fix. The user outcome is OAuth sign-in
  and invitation OAuth working from non-loopback HTTP LAN origins.
- Do not change storage keys, callback semantics, provider selection, OAuth API
  payload shape, route structure, or backend behavior as part of this bugfix.
- If any planned verification cannot run, the executor must escalate the blocker
  in-session. Silent carry-forward is not permitted.

## Out of scope

- Migrating the dev server to HTTPS (separate infra concern).
- Changes to the OAuth flow beyond the SHA-256 digest step.
- `crypto.getRandomValues()` — this is available in all contexts and is not
  affected.

## Approach

Fix PKCE challenge generation at the source: `remote-frontend/src/pkce.ts`.
`generateChallenge(verifier)` must remain the single public API used by the
login and invitation flows, but it must no longer assume `crypto.subtle` exists.

Implementation is direct and complete:

1. Keep `generateVerifier()` unchanged: it uses `crypto.getRandomValues()` to
   create a 32-byte verifier and base64url-encodes it. This API remains
   available in insecure contexts and is not the failing code path.
2. Change `generateChallenge(verifier)` so it delegates to an internal
   `sha256(bytes)` helper.
3. Implement `sha256(bytes)` with two branches:
   native first: if `globalThis.crypto?.subtle?.digest` is a function, call
   `crypto.subtle.digest('SHA-256', bytes)` and return the digest bytes;
   fallback second: otherwise run an in-repo SHA-256 implementation over the
   UTF-8 bytes and return the same 32 digest bytes.
4. Keep the current challenge encoding as lowercase hexadecimal because the
   current backend/API contract expects the app challenge string produced by
   `bytesToHex()`. This workstream fixes the broken digest primitive without
   changing the challenge wire format.
5. Add direct unit tests in `remote-frontend/src/pkce.test.ts` for native and
   fallback SHA-256 paths using known vectors, including `abc` and the empty
   string.
6. Add route-level tests for `/login` and `/invitations/:token/accept` so both
   OAuth entry points prove that sign-in reaches `initOAuth()` with a valid
   challenge instead of throwing before the OAuth request starts.
7. Preserve the existing callback and storage behavior: `storeVerifier()`,
   `retrieveVerifier()`, `storeInvitationToken()`, and
   `retrieveInvitationToken()` keep their current keys and semantics.
8. Run the full repository gate. Completion is blocked until every gate is
   green.

There is no reduced version of this plan. A solution that only adds a fallback
without proving both OAuth entry points work is incomplete. A solution that only
adds tests without manual LAN verification is incomplete. A solution that passes
remote-frontend tests but skips the repository gate is incomplete.

## Design / architecture

### Current failure path

- `remote-frontend/src/AppRouter.tsx:38-39` calls `generateVerifier()` and
  `generateChallenge()` before `initOAuth()` on the normal login page.
- `remote-frontend/src/pages/InvitationPage.tsx:31-32` calls the same PKCE
  helpers before invitation OAuth.
- `remote-frontend/src/pkce.ts:10` currently executes
  `crypto.subtle.digest('SHA-256', data)` unconditionally.
- On non-loopback HTTP origins, browsers expose `crypto.getRandomValues()` but
  do not expose `crypto.subtle`; the unconditional access throws before either
  OAuth entry point can call `initOAuth()`.

### Required user-flow preservation

The implementation must preserve the existing flow boundaries exactly:

- `/login` continues to call `generateVerifier()`, `generateChallenge()`,
  `storeVerifier()`, then `initOAuth(provider, callbackUrl, challenge)` before
  assigning `window.location` to the returned provider URL.
- `/oauth/callback` continues to call `retrieveVerifier()`, redeem the handoff
  with `oauthApi.redeem(handoffId, appCode, appVerifier)`, store the access
  token, clear the verifier, and redirect to the safe return target.
- `/invitations/:token/accept` continues to call `generateVerifier()`,
  `generateChallenge()`, `storeVerifier()`, `storeInvitationToken(token)`, then
  `initOAuth(provider, returnTo, challenge)` before assigning `window.location`
  to the returned provider URL.
- `/invitations/:token/complete` and any downstream invitation completion code
  continue to read the same invitation token key. This workstream does not
  rename or migrate session storage keys.

### Target module shape

`remote-frontend/src/pkce.ts` owns all PKCE primitives:

```ts
export function generateVerifier(): string
export async function generateChallenge(verifier: string): Promise<string>

function sha256(data: Uint8Array): Promise<Uint8Array>
function sha256Fallback(data: Uint8Array): Uint8Array
function bytesToHex(bytes: Uint8Array): string
function base64UrlEncode(array: Uint8Array): string
```

`generateChallenge()` encodes the verifier using `TextEncoder`, passes the bytes
to `sha256()`, then returns `bytesToHex(digest)`. Call sites do not change.

`sha256()` is the environment boundary. It checks capability, not origin string:

```ts
const subtleDigest = globalThis.crypto?.subtle?.digest
if (typeof subtleDigest === 'function') {
  return new Uint8Array(await subtleDigest.call(globalThis.crypto.subtle, 'SHA-256', data))
}
return sha256Fallback(data)
```

Capability detection is required because test runners, embedded browsers, and
future browser changes may not line up exactly with `window.location.protocol`.

`sha256Fallback()` is a compact, deterministic SHA-256 implementation in
TypeScript. It must implement the standard SHA-256 preprocessing and compression
steps:

- append `0x80` after the message;
- pad with zero bytes until the message length is congruent to 56 modulo 64;
- append the original bit length as a 64-bit big-endian integer;
- initialize the eight SHA-256 hash words;
- expand the 16 input words to the 64-word message schedule;
- run the 64 SHA-256 rounds with the standard constants;
- emit the eight final words as 32 big-endian bytes.

The fallback handles the verifier sizes used here and general UTF-8 input. It
must not use Node-only APIs (`Buffer`, `node:crypto`) because this code runs in
the browser bundle.

### Tests and fixtures

Add `remote-frontend/src/pkce.test.ts` with direct tests:

- native digest branch: install a fake `crypto.subtle.digest` that returns a
  known `ArrayBuffer`, assert `generateChallenge()` returns the expected hex,
  and assert the digest was called with `SHA-256` and UTF-8 verifier bytes;
- fallback digest branch: expose `crypto` without `subtle`, assert known SHA-256
  vectors:
  `generateChallenge('') === e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
  and
  `generateChallenge('abc') === ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`;
- fallback browser-compatibility branch: expose `crypto.getRandomValues` only,
  prove `generateVerifier()` still works and `generateChallenge(verifier)`
  returns 64 lowercase hex characters.

Add route-level tests to existing frontend test files or nearby new tests:

- `/login`: render the login route, remove `crypto.subtle`, click a provider,
  and assert `initOAuth()` is called once with a 64-character lowercase hex
  challenge and no error card is shown;
- `/invitations/:token/accept`: mock `getInvitation()`, remove `crypto.subtle`,
  click a provider, and assert `initOAuth()` is called once with the expected
  invitation return URL and a 64-character lowercase hex challenge.
- callback/storage preservation: verify normal login still stores the verifier
  before redirect, invitation login still stores both verifier and invitation
  token before redirect, and the callback still redeems with the stored verifier.

All tests must restore `globalThis.crypto`, `window.location.assign`, storage,
and mocks after each case so they do not contaminate unrelated frontend tests.

## Decisions

- **Use capability detection, not origin detection.** The runtime condition is
  whether `crypto.subtle.digest` exists. This directly matches the failing API
  and works in browsers, jsdom, and embedded environments.
- **Keep the existing hex challenge payload shape.** The current implementation
  returns lowercase hex via `bytesToHex()`, and the backend already accepts that
  shape. This workstream keeps the OAuth/PKCE request contract as-is.
- **Implement the fallback in `pkce.ts`, not as a dependency.** SHA-256 is small
  enough here, dependency-free code avoids bundle and supply-chain churn, and it
  keeps the browser-only constraint explicit.
- **Exercise both entry points, not only the helper.** A helper-only fix would
  miss regressions where `/login` or invitation acceptance still fails before
  `initOAuth()`.
- **Treat the user-visible LAN sign-in outcome as the acceptance boundary.** The
  internal SHA-256 fallback is only a means to that outcome; the workstream is
  not accepted until login and invitation OAuth both work from non-loopback HTTP.
- **No planned work may be deferred.** The fallback implementation, tests,
  route coverage, storage preservation, static checks, repository gates, and LAN
  manual checks are all part of this workstream.
- **No ADR-worthy product decision is present.** This workstream adds a browser
  fallback implementation and tests while keeping persisted state keys, public
  function names, backend payload shape, and route behavior as-is. No ADR is
  required.

## Test strategy

The test strategy is mandatory and complete; no test item is deferred.

1. Run targeted frontend tests while implementing:
   `cd remote-frontend && npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx`.
2. Run the whole remote frontend test suite:
   `cd remote-frontend && npm run test:run`.
3. Run remote frontend static checks:
   `cd remote-frontend && npm run lint` and `cd remote-frontend && npx tsc --noEmit`.
4. Run the repository mandatory gate from `AGENTS.md`:
   `cargo clippy --all --all-targets --all-features -- -D warnings`,
   `cargo test --workspace`, `cd frontend && npm run lint`, and
   `cd frontend && npx tsc --noEmit`.
5. Manually verify the LAN case after tests are green: serve the remote frontend
   over a non-loopback HTTP origin, open `http://<lan-ip>:3002/login`, click an
   OAuth provider, and verify the app reaches the provider authorization URL
   instead of showing an `OAuth init failed` error caused by `crypto.subtle`.
6. Manually verify invitation OAuth over the same origin by opening
   `http://<lan-ip>:3002/invitations/<token>/accept`, clicking a provider, and
   verifying the app reaches the provider authorization URL with no digest
   exception.
7. If any check in this list is red, unavailable, or inconclusive, stop and
   resolve it or escalate immediately. Do not mark the workstream shipped, do
   not create a "later" follow-up, and do not summarize the item as complete.
