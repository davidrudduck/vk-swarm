---
doc_type: decisions-ledger
workstream: fix-nonloopback-signin
---

# Decisions ledger — fix-nonloopback-signin

## Precheck findings

### 2026-07-08 — anchor-check false positive for nested package paths

The first `/wai:precheck fix-nonloopback-signin` run that reached anchor grounding stopped on
paths extracted as root `src/...` anchors:

- `src/AppRouter.tsx`
- `src/pages/InvitationPage.tsx`
- `src/pkce.test.ts`
- `src/pkce.ts`

This is a false positive. The spec intentionally references files under the
`remote-frontend` package, such as `remote-frontend/src/pkce.ts`, but the anchor extractor scans
for `(src|extensions|ui|packages|apps)/...` substrings and strips the package prefix from nested
paths.

Evidence against `main`:

```bash
git cat-file -e main:remote-frontend/src/pkce.ts
git cat-file -e main:remote-frontend/src/AppRouter.tsx
git cat-file -e main:remote-frontend/src/pages/InvitationPage.tsx
```

All three real repo-root anchors exist on `main` (exit 0). The extracted root paths do not exist:

```text
fatal: path 'src/pkce.ts' does not exist in 'main'
fatal: path 'src/AppRouter.tsx' does not exist in 'main'
fatal: path 'src/pages/InvitationPage.tsx' does not exist in 'main'
```

Resolution: keep the spec's precise `remote-frontend/src/...` anchors and rerun precheck with the
script's explicit false-positive escape hatch: `--no-anchor-check`.

## Decompose findings

### 2026-07-08 — plan-lint sibling advisories acknowledged

`wai-plan-lint.sh fix-nonloopback-signin` passed with three advisory `W:` sibling warnings:

- Task 101 creates `remote-frontend/src/pkce.test.ts` beside unlisted sibling
  `remote-frontend/src/toolchain.test.ts`. This is not a pattern sibling: `toolchain.test.ts`
  verifies toolchain wiring, while `pkce.test.ts` is a colocated unit test for the PKCE helper.
  The task already lists and requires reading `remote-frontend/src/setupTests.ts` and
  `remote-frontend/src/api.ts` as the relevant same-directory context.
- Task 202 creates `remote-frontend/src/pages/InvitationPage.test.tsx` beside unlisted sibling
  `remote-frontend/src/pages/InvitationCompletePage.tsx`. The task's purpose is to create tests
  for the invitation flow and it already lists the production completion page test target separately
  as `remote-frontend/src/pages/InvitationCompletePage.test.tsx` plus the page-test pattern sibling
  `remote-frontend/src/pages/Nodes.test.tsx`.
- Task 202 creates `remote-frontend/src/pages/InvitationCompletePage.test.tsx` beside unlisted
  sibling `remote-frontend/src/pages/InvitationCompletePage.tsx`. This is the production file under
  test, but task 202 is create-only and must not edit production code. The test task is constrained
  to create the new test files and read the page/test siblings.

No advisory indicates a missing implementation task or deferred work. The plan-lint hard gate passes
with full SC coverage.

### 2026-07-08 — tournament round 1 closure

The required breakdown tournament ran with Codex, Gemini, and OpenCode/GLM-5.2 competitors.
Round record: `docs/plans/fix-nonloopback-signin/reviews/tournament-round-1.md`.

Validated remediation applied:

- Task 301 no longer has a hollow `true` gate. It now runs all automated gates through
  `WAI_TYPECHECK_CMD` and then runs `docs/plans/fix-nonloopback-signin/verify-301-evidence.sh`.
- `verify-301-evidence.sh` rejects missing acceptance evidence, missing PASS lines, placeholder
  result text, non-passing checks, missing-environment checks, and indeterminate checks.
- Tasks 201 and 202 no longer prescribe brittle jsdom `window.location.assign` spying. Their route
  tests keep `initOAuth()` pending after challenge/storage assertions, and task 301 remains the
  required browser-level proof that the provider authorization URL is reached over LAN HTTP.

Focused re-checks after remediation:

- `bash -n docs/plans/fix-nonloopback-signin/verify-301-evidence.sh` — PASS.
- `wai-plan-lint.sh fix-nonloopback-signin` — PASS with only acknowledged sibling advisories.

## Task 101 implementation findings

### 2026-07-08 — native digest argument type cast

`remote-frontend/src/pkce.ts` casts the `Uint8Array` passed to `crypto.subtle.digest` as
`BufferSource`. This preserves the runtime value asserted by `pkce.test.ts` while satisfying the
repo's DOM typings, which reject `Uint8Array<ArrayBufferLike>` as a `BufferSource` under
`npx tsc --noEmit`.

## Task 201 implementation findings

### 2026-07-08 — partial api mock preserves invitation route test

`remote-frontend/src/AppRouter.test.tsx` uses a partial `@/api` mock that overrides only
`initOAuth`. The literal full replacement mock from task 201 hid the existing `getInvitation`
export used by `InvitationPage`, causing the pre-existing invitation route test to render React
Router's error boundary instead of the loading or invitation state.

## Reachability gate

### (a) CALL-PATH TRACE

Entry points and bug path traced through real merged code:

1. `/login` entry: `remote-frontend/src/AppRouter.tsx:39` calls `generateChallenge(verifier)`.
2. `generateChallenge()` → `remote-frontend/src/pkce.ts:10-12` calls `sha256(data)`.
3. `sha256()` at `remote-frontend/src/pkce.ts:14-20` checks `globalThis.crypto?.subtle`. On non-secure HTTP LAN origins (`window.isSecureContext=false`), `crypto.subtle` is `undefined` → falls through to `sha256Fallback()` at line 24.
4. `sha256Fallback()` returns correct SHA-256 bytes → `bytesToHex()` → 64-char lowercase hex challenge.
5. Challenge flows to `oauthApi.init(provider, returnTo, challenge)` at `AppRouter.tsx:47` → `oauth.ts:23` POST `/v1/oauth/web/init`.
6. Same path for invitation: `InvitationPage.tsx:32` calls `generateChallenge()` → same `pkce.ts` fallback path.

Bug path confirmed: `pkce.ts:14-20` (capability-detected fallback) executes on every non-secure origin call to `generateChallenge()`.

### (b) REAL-SEAM TEST

- `remote-frontend/src/AppRouter.test.tsx` drives the real `/login` route via `createMemoryRouter(createRoutes())` (line 61-63) with `crypto.subtle` removed (line 30-38), clicks the real GitHub button (line 102), and asserts `initOAuth` receives a 64-char hex challenge (lines 106-112). This proves the route reaches the changed PKCE code, not a mock past it.
- `remote-frontend/src/pages/InvitationPage.test.tsx` drives the real `/invitations/:token/accept` route with `crypto.subtle` removed (lines 13-21, 60), clicks the real GitHub button (line 64), and asserts `initOAuth` receives a 64-char hex challenge (lines 66-72).
- Manual Playwright LAN verification: `http://10.69.96.233:3002/login` navigated to `https://provider.test/authorize?flow=login` with `crypto.subtle` absent and no local error. `http://10.69.96.233:3002/invitations/invite-token/accept` navigated to `https://provider.test/authorize?flow=invitation` with same result.

### (c) INCIDENT-SYMPTOM ASSERTION

The documented incident symptom (F-2026-07-06-02): "Sign-in broken on non-loopback HTTP origins (crypto.subtle undefined)" at `remote-frontend/src/pkce.ts:10`.

Behavioural assertion mapped to symptom: on a non-secure HTTP LAN origin, clicking a provider button on `/login` or `/invitations/:token/accept` reaches the provider authorization URL without throwing a `crypto.subtle` digest error. Both unit tests and manual Playwright verification prove this directly — the old `crypto.subtle.digest()` call at `pkce.ts:10` would have thrown `Cannot read properties of undefined (reading 'digest')` before `initOAuth()` could be called.

Result: all three reachability gate requirements (a), (b), (c) pass.

VERDICT: PASS

## Acceptance evidence

### Task 301 — full gates and LAN OAuth verification

Automated gates:

- `cd remote-frontend && npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx` — PASS, 4 files and 12 tests passed.
- `cd remote-frontend && npm run test:run` — PASS, 25 files and 114 tests passed.
- `cd remote-frontend && npm run lint` — PASS, eslint exited 0.
- `cd remote-frontend && npx tsc --noEmit` — PASS, exited 0 with no diagnostics.
- `cargo clippy --all --all-targets --all-features -- -D warnings` — PASS, exited 0.
- `cargo test --workspace` — PASS, exited 0 across workspace crates and doctests.
- `cd frontend && npm run lint` — PASS, eslint exited 0 after installing frontend dependencies with the repo package manager (`pnpm install --frozen-lockfile`).
- `cd frontend && npx tsc --noEmit` — PASS, exited 0 with no diagnostics.

Manual LAN verification:

- Normal login over `http://10.69.96.233:3002/login`: provider button clicked, provider authorization URL `https://provider.test/authorize?flow=login` reached, `window.isSecureContext=false`, `crypto.subtle` absent, OAuth init sent a 64-character lowercase hex `app_challenge`, no local `crypto.subtle` error shown — PASS.
- Invitation OAuth over `http://10.69.96.233:3002/invitations/invite-token/accept`: provider button clicked, provider authorization URL `https://provider.test/authorize?flow=invitation` reached, `window.isSecureContext=false`, `crypto.subtle` absent, invitation was fetched, OAuth init sent return URL `http://10.69.96.233:3002/invitations/invite-token/complete` and a 64-character lowercase hex `app_challenge`, no local `crypto.subtle` error shown — PASS.

Result: PASS.

### Task 301 — pre-existing full-suite gate repair

`cd remote-frontend && npm run test:run` initially collected `remote-frontend/scripts/no-push-invariant.test.mjs` as a Vitest suite. That file intentionally uses Node's `node:test` runner and is documented in `dev-docs/workstreams/vk-swarm-hive-ui/plans/vk-swarm-hive-ui/phase-3/308-no-push-invariant.md` with the command `cd remote-frontend && node --test scripts/no-push-invariant.test.mjs`.

Root repair: `remote-frontend/vite.config.ts` now excludes `scripts/**` from Vitest collection. Verification:

- `cd remote-frontend && npm run test:run` — PASS, 25 files and 114 tests passed.
- `cd remote-frontend && node --test scripts/no-push-invariant.test.mjs` — PASS, 1 node-test passed.
