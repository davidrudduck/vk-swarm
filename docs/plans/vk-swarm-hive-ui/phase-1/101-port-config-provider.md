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
irreversible: false
scope_test: "remote-frontend/src/components/ProfileProvider.test.tsx"
allowed_change: create
covers_criteria: [SC3]
---
## Failing test (write first)
File: `remote-frontend/src/components/ProfileProvider.test.tsx`

Test renders `ProfileProvider`, mocks `fetch` to return a `ProfileResponse` (200), and asserts the context exposes `{ profile, isSignedIn: true, isLoaded: true }`. A second test mocks `fetch` to 401 and asserts `{ profile: null, isSignedIn: false, isLoaded: true }`. A third asserts `isLoaded: false` before the fetch resolves.

Use `@testing-library/react` `renderHook` + a `useProfile()` consumer hook. Mock global `fetch` (the hive server is not running in unit tests).

## Change
- **File:** `remote-frontend/src/components/ProfileProvider.tsx` (CREATE)
  - **Anchor:** new file
  - **Before:** (file does not exist)
  - **After:** A `ProfileProvider` React context that:
    - Defines `ProfileResponse` type: `{ user_id: string; username: string | null; email: string; providers: Array<{ provider: string; username: string | null; display_name: string | null; email: string | null; avatar_url: string | null }> }` (mirror of `crates/utils/src/api/oauth.rs:49-63` `ProfileResponse` — `username` and all `ProviderProfile` fields are `Option<String>` in Rust, so nullable in TS).
    - Defines `ProfileState` type: `{ profile: ProfileResponse | null; isSignedIn: boolean; isLoaded: boolean }`.
    - Creates `ProfileContext` with default `{ profile: null, isSignedIn: false, isLoaded: false }`.
    - `ProfileProvider` component: on mount, `fetch('/v1/profile', { credentials: 'include' })`. On 200 → `setProfile(data); setIsSignedIn(true); setIsLoaded(true)`. On 401/other → `setProfile(null); setIsSignedIn(false); setIsLoaded(true)`. No retry loop (hive is the only backend; if `/v1/profile` is down, the whole app is down — unlike the node `ConfigProvider` which retries because the node server may still be starting). The `/v1` prefix is required — all hive routes nest under `/v1` per `crates/remote/src/routes/mod.rs:112-113`.
    - Exports `useProfile()` hook: `useContext(ProfileContext)`; throws if used outside `ProfileProvider`.
  - **Sibling alignment:** Read `frontend/src/components/ConfigProvider.tsx`. It is the pattern source — a React context wrapping a fetch-on-mount + state. Divergences from it are intentional and recorded in the decisions ledger:
    1. `ProfileProvider` fetches `/v1/profile` (hive identity, under the `/v1` mount), NOT `/api/config` (node `UserSystemInfo` — the hive does not serve this route).
    2. `ProfileState` has 3 fields (`profile`, `isSignedIn`, `isLoaded`), NOT `UserSystemState`'s 6 (`config`, `environment`, `profiles`, `capabilities`, `analytics_user_id`, `login_status`). The hive has no config/environment/profiles/capabilities surface.
    3. No retry loop — `ConfigProvider` retries because the node server may still be booting; the hive app shell assumes the hive is up (single-hive deployment).
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