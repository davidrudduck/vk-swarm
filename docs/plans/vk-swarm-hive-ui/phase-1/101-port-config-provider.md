---
id: "101"
phase: 1
title: "Hive app shell: ProfileProvider context (auth state from /profile)"
status: ready
depends_on: ["100"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/ConfigProvider.tsx
  - remote-frontend/src/components/ProfileProvider.tsx
  - remote-frontend/src/components/ProfileProvider.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/ProfileProvider.test.tsx"
allowed_change: edit
covers_criteria: [SC3]
---
## Token storage contract (orchestrator-level, binds tasks 101, 102, 105)

The hive uses **Bearer token auth**, NOT cookies. Evidence: `crates/remote/src/auth/middleware.rs:46-53` — `require_session` reads only the `Authorization<Bearer>` header; `grep -rn 'cookie|Cookie' crates/remote/src/` returns zero matches. The existing `remote-frontend/src/api.ts:85` already sends `Authorization: Bearer ${accessToken}`.

**Storage:** the access_token is stored in `localStorage` under key `access_token`. This matches the existing invitation flow (`remote-frontend/src/api.ts:77-91` `acceptInvitation(token, accessToken)` — the caller already has an `accessToken` in hand).

**Binding:**
- **Task 101 (this task):** `ProfileProvider` reads `localStorage.getItem('access_token')`. If absent → signed-out state (`{ profile: null, isSignedIn: false, isLoaded: true }`), NO fetch. If present → `fetch('/v1/profile', { headers: { Authorization: 'Bearer ' + token } })`. On 401 → clear `localStorage.removeItem('access_token')` (token expired/invalid) and set signed-out.
- **Task 102:** `profileApi.get()` uses the same `Authorization: Bearer <localStorage>` header (NOT `credentials: include`). `oauthApi.logout()` clears `localStorage.removeItem('access_token')` and POSTs `/v1/oauth/logout` with the Bearer header.
- **Task 105:** the `/oauth/callback` route handler calls `oauthApi.redeem(...)`, stores the returned `access_token` into `localStorage.setItem('access_token', token)`, then redirects to `/nodes`.

## Failing test (write first)
File: `remote-frontend/src/components/ProfileProvider.test.tsx`

Tests (use `@testing-library/react` `renderHook` + a `useProfile()` consumer hook; mock global `fetch` and `localStorage`):
1. **Signed-in path:** `localStorage.getItem('access_token')` returns a token; `fetch` mocked to 200 with a `ProfileResponse` body → context exposes `{ profile, isSignedIn: true, isLoaded: true }`. Assert `fetch` was called with `/v1/profile` and header `Authorization: Bearer <token>` (NOT `credentials: 'include'`).
2. **No token → signed-out (no fetch):** `localStorage.getItem('access_token')` returns `null` → context is `{ profile: null, isSignedIn: false, isLoaded: true }` and `fetch` was NOT called (signed-out without a network round-trip).
3. **401 clears token:** `localStorage.getItem('access_token')` returns a token; `fetch` mocked to 401 → context is `{ profile: null, isSignedIn: false, isLoaded: true }` AND `localStorage.removeItem('access_token')` was called (expired token is cleared).
4. **Network error path:** `localStorage.getItem('access_token')` returns a token; `fetch` mocked to reject (network failure) → context is `{ profile: null, isSignedIn: false, isLoaded: true }` (the `catch` branch is exercised; token is NOT cleared — a transient network failure is not a token expiry).
5. **Loading state:** before the fetch resolves, `isLoaded` is `false` (only relevant in the signed-in path where fetch is called).

## Change
- **File:** `remote-frontend/src/components/ProfileProvider.tsx` (CREATE)
  - **Anchor:** new file
  - **Before:** (file does not exist)
  - **After:** A `ProfileProvider` React context that:
    - Defines `ProfileResponse` type: `{ user_id: string; username: string | null; email: string; providers: Array<{ provider: string; username: string | null; display_name: string | null; email: string | null; avatar_url: string | null }> }` (mirror of `crates/utils/src/api/oauth.rs:49-63` `ProfileResponse` — `username` and all `ProviderProfile` fields are `Option<String>` in Rust, so nullable in TS).
    - Defines `ProfileState` type: `{ profile: ProfileResponse | null; isSignedIn: boolean; isLoaded: boolean }`.
    - Creates `ProfileContext` with default `{ profile: null, isSignedIn: false, isLoaded: false }`.
    - `ProfileProvider` component: on mount, read `localStorage.getItem('access_token')`. If `null` → set `{ profile: null, isSignedIn: false, isLoaded: true }` (signed-out without fetching). If present → `fetch('/v1/profile', { headers: { Authorization: 'Bearer ' + token } })` (NOT `credentials: 'include'` — the hive auth middleware reads only the `Authorization<Bearer>` header, `crates/remote/src/auth/middleware.rs:46-53`; there is no cookie session). On 200 → `setProfile(data); setIsSignedIn(true); setIsLoaded(true)`. On 401 → `localStorage.removeItem('access_token')` (token expired/invalid) then `setProfile(null); setIsSignedIn(false); setIsLoaded(true)`. On other non-200 or network error (`catch`) → `setProfile(null); setIsSignedIn(false); setIsLoaded(true)` (do NOT clear the token on a transient network failure — only on 401). No retry loop (hive is the only backend; if `/v1/profile` is down, the whole app is down — unlike the node `ConfigProvider` which retries because the node server may still be starting). The `/v1` prefix is required — all hive routes nest under `/v1` per `crates/remote/src/routes/mod.rs:112-113`.
    - Exports `useProfile()` hook: `useContext(ProfileContext)`; throws if used outside `ProfileProvider`.
  - **Sibling alignment:** Read `frontend/src/components/ConfigProvider.tsx`. It is the pattern source — a React context wrapping a fetch-on-mount + state. Divergences from it are intentional and recorded in the decisions ledger:
    1. `ProfileProvider` fetches `/v1/profile` (hive identity, under the `/v1` mount), NOT `/api/config` (node `UserSystemInfo` — the hive does not serve this route).
    2. `ProfileState` has 3 fields (`profile`, `isSignedIn`, `isLoaded`), NOT `UserSystemState`'s 6 (`config`, `environment`, `profiles`, `capabilities`, `analytics_user_id`, `login_status`). The hive has no config/environment/profiles/capabilities surface.
     3. No retry loop — `ConfigProvider` retries because the node server may still be booting; the hive app shell assumes the hive is up (single-hive deployment).
     4. **Bearer token auth (NOT cookies)** — the hive `require_session` middleware reads only the `Authorization<Bearer>` header (`crates/remote/src/auth/middleware.rs:46-53`); there is no cookie session. The access_token is stored in `localStorage` under key `access_token` (orchestrator-level contract, see "Token storage contract" above). `ConfigProvider` relies on cookie session middleware on the node server; `ProfileProvider` cannot.
     - Record these divergences in `docs/plans/vk-swarm-hive-ui/decisions-ledger.md`.

## Allowed moves
- Create `remote-frontend/src/components/ProfileProvider.tsx` and `remote-frontend/src/components/ProfileProvider.test.tsx`.
- No changes to `frontend/src/components/ConfigProvider.tsx` (read-only sibling reference).

## STOP triggers
- If `crates/remote/src/routes/oauth.rs` `profile()` does NOT return `ProfileResponse { user_id, username, email, providers }` — STOP and record in the ledger (the spec assumes this shape; a mismatch is a spec-vs-reality contradiction requiring escalation, not a silent fix).
- If `remote-frontend/package.json` does not already include `@testing-library/react` — STOP and add it as a dev dependency first (record in ledger).

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/components/ProfileProvider.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0 (the new file typechecks).
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/ProfileProvider.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 101` exits 0
