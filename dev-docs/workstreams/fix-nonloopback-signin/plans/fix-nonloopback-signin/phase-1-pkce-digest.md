# Phase 1 — PKCE digest primitive

Fix the root browser compatibility bug at `remote-frontend/src/pkce.ts`. This phase is shippable when `generateChallenge()` returns correct SHA-256 lowercase hex with native `crypto.subtle` and with only `crypto.getRandomValues` available.

Tasks:

- `101` — Implement browser-safe SHA-256 fallback for PKCE.

Exit criteria:

- `remote-frontend/src/pkce.test.ts` proves native and fallback digest behavior.
- `remote-frontend/src/pkce.ts` still exports the same public functions.
- No OAuth call sites change in this phase.
