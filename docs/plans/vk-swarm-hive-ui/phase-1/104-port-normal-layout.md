---
id: "104"
phase: 1
title: "Hive app shell: NormalLayout (Navbar + Outlet + BottomNav) ported"
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/layout/NormalLayout.tsx
  - frontend/src/components/layout/Navbar.tsx
  - frontend/src/components/layout/BottomNav.tsx
  - remote-frontend/src/components/layout/NormalLayout.tsx
  - remote-frontend/src/components/layout/Navbar.tsx
  - remote-frontend/src/components/layout/BottomNav.tsx
irreversible: false
scope_test: "remote-frontend/src/components/layout/NormalLayout.test.tsx"
allowed_change: create
covers_criteria: [SC3]
---
## Failing test (write first)
File: `remote-frontend/src/components/layout/NormalLayout.test.tsx`

Renders `NormalLayout` with a child `<div data-testid="outlet-child" />` as the `Outlet` content. Asserts:
1. A nav element renders (role="navigation" or a `[data-testid="navbar"]`).
2. The outlet child renders inside the layout.
3. The bottom nav renders.

Mock `useProfile()` to return `{ isSignedIn: true, isLoaded: true }` (the layout may gate nav visibility on auth state — verify against the node `NormalLayout` and mirror its behaviour, or simplify: always show nav in the hive shell since the router gates auth at the route level).

## Change
- **File:** `remote-frontend/src/components/layout/NormalLayout.tsx` (CREATE)
  - **Before:** (file does not exist)
  - **After:** Port `frontend/src/components/layout/NormalLayout.tsx` structure: a layout component with Navbar + `<Outlet />` + BottomNav. Drop the `view=preview|diffs` navbar-hiding logic IF the hive app shell has no preview/diffs view (it doesn't — that's a node-frontend local-UI concept). Drop `DevBanner` IF it depends on `useUserSystem()` (the hive has no `ConfigProvider`); either drop it or reimplement against `useProfile()`. Keep `Navbar` and `BottomNav` components — port them too if they are separate files in `frontend/src/components/layout/`.
  - **Sibling alignment:** Read `frontend/src/components/layout/NormalLayout.tsx` AND every component it imports (Navbar, BottomNav, DevBanner). List every exclusion, guard, and structural choice the node version makes. Justify each divergence in the ledger:
    1. Drop `view=preview|diffs` hiding (hive has no preview/diffs route).
    2. Drop or reimplement `DevBanner` (depends on `ConfigProvider` context the hive doesn't have).
    3. Nav items differ (hive console has `/nodes`, `/tasks`, `/settings` — not the node's `/projects`, `/processes`, etc.). The nav item list is hive-specific.

## Allowed moves
- Create `remote-frontend/src/components/layout/NormalLayout.tsx` + any sub-components it needs (Navbar, BottomNav) + the test file.
- Read-only reference to `frontend/src/components/layout/NormalLayout.tsx` and its imports.

## STOP triggers
- If `NormalLayout` imports `useUserSystem` or `ConfigProvider` for anything other than `DevBanner` (which we drop/reimplement) — STOP; that dependency belongs to the node-frontend's config surface and must not be dragged into the hive shell. Record in the ledger.
- If the node `Navbar`/`BottomNav` are not separate files (inline in NormalLayout) — port them inline; no STOP needed, just note it.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/components/layout/NormalLayout.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/layout/NormalLayout.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 104` exits 0