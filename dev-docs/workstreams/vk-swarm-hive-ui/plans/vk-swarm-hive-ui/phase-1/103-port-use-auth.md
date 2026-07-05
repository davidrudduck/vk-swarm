---
id: "103"
phase: 1
title: "Hive app shell: useAuth hook over ProfileProvider"
status: done
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - frontend/src/hooks/auth/useAuth.ts
  - remote-frontend/src/hooks/auth/useAuth.ts
  - remote-frontend/src/hooks/auth/useAuth.test.tsx
irreversible: false
scope_test: "remote-frontend/src/hooks/auth/useAuth.test.tsx"
allowed_change: create
covers_criteria: [SC3]
---
## Failing test (write first)
File: `remote-frontend/src/hooks/auth/useAuth.test.tsx`

Wraps a consumer component in `ProfileProvider` (mocked fetch → 200 + `ProfileResponse`), calls `useAuth()`, asserts `{ isSignedIn: true, isLoaded: true, userId: <profile.user_id> }`. A second test with fetch → 401 asserts `{ isSignedIn: false, isLoaded: true, userId: null }`. A third test that `useAuth()` called outside `ProfileProvider` throws.

## Change
- **File:** `remote-frontend/src/hooks/auth/useAuth.ts` (CREATE)
  - **Before:** (file does not exist)
  - **After:** `useAuth()` hook: calls `useProfile()` (from task 101's `ProfileProvider`), returns `{ isSignedIn: profileState.isSignedIn, isLoaded: profileState.isLoaded, userId: profileState.profile?.user_id ?? null }`.
  - **Sibling alignment:** Read `frontend/src/hooks/auth/useAuth.ts`. It wraps `useUserSystem()` (from `ConfigProvider`) and exposes `isSignedIn`, `isLoaded`, `userId`. The hive `useAuth` wraps `useProfile()` (from `ProfileProvider`) — same surface, different context source. Record the divergence in the ledger (context source = `ProfileProvider` vs `ConfigProvider`; field names identical on the hook surface, which is the point — the consumer components don't need to change).

## Allowed moves
- Create `remote-frontend/src/hooks/auth/useAuth.ts` and the test file.
- Read-only reference to `frontend/src/hooks/auth/useAuth.ts`.

## STOP triggers
- If task 101's `ProfileProvider` does not export `useProfile` with the shape `{ profile, isSignedIn, isLoaded }` — STOP; fix task 101 first (it is in `depends_on`).

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/hooks/auth/useAuth.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/hooks/auth/useAuth.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 103` exits 0