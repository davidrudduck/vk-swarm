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
  - remote-frontend/package.json
  - remote-frontend/src/lib/utils.ts
  - remote-frontend/src/components/layout/NormalLayout.tsx
  - remote-frontend/src/components/layout/Navbar.tsx
  - remote-frontend/src/components/layout/BottomNav.tsx
  - remote-frontend/src/components/layout/NormalLayout.test.tsx
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
This task ports a **SLIM** hive shell layout. The node `frontend/src/components/layout/Navbar.tsx` is 417 lines pulling in ~15 deps the hive `remote-frontend/` does not have (DropdownMenu/Dialog/Tooltip from `@radix-ui`, `OAuthDialog`, `ActivityFeed`, `ProjectSwitcher`, `OpenInIdeButton`, `ThemeToggle`, `SearchBar`, `useProject`, `useSearch`, `useUserSystem`, i18next, a `Button` UI primitive). Porting verbatim is out of scope — the task ports only the STRUCTURE (logo + nav links + logout), not the node's full nav surface.

- **File:** `remote-frontend/package.json` (EDIT) — add `lucide-react` (same major as node frontend: `^1.7.0`) to `dependencies`. Run `npm install` after.
- **File:** `remote-frontend/src/lib/utils.ts` (CREATE) — port the `cn` helper from `frontend/src/lib/utils.ts` verbatim (`clsx` + `tailwind-merge` already in deps).
- **File:** `remote-frontend/src/components/layout/NormalLayout.tsx` (CREATE)
  - **Before:** (file does not exist)
  - **After:** Port `frontend/src/components/layout/NormalLayout.tsx` structure: `<> <Navbar /> <div className="flex-1 min-h-0 overflow-hidden pb-14 sm:pb-0"><Outlet /></div> <BottomNav /> </>`. Drop the `view=preview|diffs` navbar-hiding logic (hive has no preview/diffs). Drop `DevBanner` (depends on `useUserSystem()` the hive doesn't have).
- **File:** `remote-frontend/src/components/layout/Navbar.tsx` (CREATE)
  - **Before:** (file does not exist)
  - **After:** SLIM hive Navbar (~40-60 lines, NOT the node's 417). Top row: a text logo `<Link to="/nodes">VK Swarm</Link>` (no VKSLogo component). Second row: nav links for the hive console — `Nodes` → `/nodes`, `Tasks` → `/tasks`, `Settings` → `/settings` (matching `INTERNAL_NAV` pattern from node Navbar.tsx:51-54 but hive routes). A `Logout` button (calls `oauthApi.logout()` from `@/lib/api/oauth`, then `window.location.reload()` — the node's `reloadSystem()` lives on `useUserSystem` which the hive doesn't have). Use `lucide-react` icons (`FolderOpen`, `ListTodo`, `Settings`, `LogOut`) for parity with node BottomNav style. Use `cn` from `@/lib/utils`. Use `useLocation` for active-link highlighting (mirror node Navbar.tsx:344-373 border-b-2 pattern). No search bar, no project switcher, no archive toggle, no activity feed, no OAuth dialog, no dropdown menu — all out of scope for the hive shell v1.
- **File:** `remote-frontend/src/components/layout/BottomNav.tsx` (CREATE)
  - **Before:** (file does not exist)
  - **After:** SLIM hive BottomNav (~40 lines, NOT the node's 118). Fixed bottom nav, `sm:hidden` (mobile only, same as node). 3 items: `Nodes` (FolderOpen icon, `/nodes`), `Tasks` (ListTodo icon, `/tasks`), `Settings` (Menu/Settings icon, `/settings`). No `useProject`, no `openTaskForm`, no i18next. Use `cn` + `lucide-react`. The `NavItem` sub-component from node BottomNav.tsx:8-33 can be reused verbatim (it's self-contained: icon + label + isActive + onClick).
  - **Sibling alignment:** Read `frontend/src/components/layout/{NormalLayout,Navbar,BottomNav}.tsx`. List every exclusion, guard, and structural choice the node version makes. Justify each divergence in the ledger:
    1. Drop `view=preview|diffs` hiding (hive has no preview/diffs route).
    2. Drop `DevBanner` (depends on `ConfigProvider` context the hive doesn't have).
    3. Nav items differ (hive console has `/nodes`, `/tasks`, `/settings` — not the node's `/projects`, `/processes`, etc.). The nav item list is hive-specific.
    4. SLIM Navbar (40-60 lines vs node 417): dropped search, project switcher, archive toggle, activity feed, OAuth dialog, dropdown menu, `useProject`, `useSearch`, `useUserSystem`, `OpenInIdeButton`, `ThemeToggle`, i18next. These are node-frontend local-UI concepts not applicable to the hive console v1.
    5. SLIM BottomNav (40 lines vs node 118): dropped `useProject`, `openTaskForm`, i18next. 3 hive nav items instead of 5 node items.
    6. Logout: `oauthApi.logout()` + `window.location.reload()` (hive has no `reloadSystem()` — the node's lives on `useUserSystem`).

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