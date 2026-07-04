---
id: "105"
phase: 1
title: "Hive app shell: replace 4-page auth stub AppRouter with full console route tree"
status: ready
depends_on: ["101", "102", "103", "104"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/AppRouter.tsx
  - remote-frontend/src/App.tsx
irreversible: false
scope_test: "remote-frontend/src/AppRouter.test.tsx"
allowed_change: edit
covers_criteria: [SC3]
---
## Failing test (write first)
File: `remote-frontend/src/AppRouter.test.tsx`

Uses `createMemoryRouter` (or `render` with a MemoryRouter) to test routes:
1. Unauthenticated (`useProfile` mocked → `{ isSignedIn: false, isLoaded: true }`): hitting `/` redirects to `/login` (or the OAuth provider picker `/oauth`).
2. Authenticated (`useProfile` mocked → `{ isSignedIn: true, isLoaded: true, profile }`): hitting `/` redirects to `/nodes`.
3. Authenticated: hitting `/nodes` renders the Nodes page (lazy-loaded placeholder OK for this task — just assert the route resolves and the layout renders).
4. Hitting `/invitations/:id` still renders `InvitationPage` (the invitation flow is preserved from the auth stub).
5. Unknown path renders `NotFoundPage`.

## Change
- **File:** `remote-frontend/src/AppRouter.tsx` (EDIT — replace)
  - **Anchor:** the existing `createBrowserRouter` call with 4 routes.
  - **Before:** the current 4-route router (`HomePage`, `InvitationPage`, `InvitationCompletePage`, `NotFoundPage`).
  - **After:** a full route tree:
    - `/` — index, redirects to `/nodes` if authenticated, `/login` if not.
    - `/login` — OAuth provider picker page (uses `oauthApi.init` from task 102).
    - `/oauth/callback` — OAuth callback handler: reads `handoff_id`, `app_code` (the authorization code returned by the provider), and the stored PKCE `app_verifier` from the location (query string or router state — record which in the ledger); calls `oauthApi.redeem(handoffId, appCode, appVerifier)`; stores the returned `access_token` via `localStorage.setItem('access_token', token)`; then redirects to `/nodes` (or `returnTo` if present). On redeem failure → redirect to `/login` with an error query param.
    - `/invitations/:id` — `InvitationPage` (preserved from auth stub).
    - `/invitations/:id/complete` — `InvitationCompletePage` (preserved).
    - `/nodes` — lazy `Nodes` page (rehosted in phase 2; for now a placeholder `<div>Nodes (coming in phase 2)</div>` is fine — the route exists so the shell is testable).
    - `/tasks` — lazy `Tasks` page (placeholder for phase 3).
    - `*` — `NotFoundPage` (preserved).
  - Wrap authenticated routes in `NormalLayout` (task 104). The `/login`, `/oauth/callback`, and `/invitations/*` routes render WITHOUT `NormalLayout` (they are pre-auth / out-of-band flows).
  - Use `createBrowserRouter` (keep the existing router type) or switch to `createMemoryRouter` for testability — record the choice in the ledger.

- **File:** `remote-frontend/src/App.tsx` (EDIT)
  - **Anchor:** the current `<AppRouter />` render and any placeholder text.
  - **Before:** `App.tsx` renders a "Frontend coming soon..." placeholder + `<AppRouter />`.
  - **After:** `App.tsx` wraps `<AppRouter />` in `<ProfileProvider>` (task 101) and `<QueryClientProvider>` (the rehosted swarm components in phase 2 need react-query; add it now so the shell is complete). Remove the placeholder text. Keep any existing providers (i18n if present — check `remote-frontend/src/`).

## Allowed moves
- Edit `remote-frontend/src/AppRouter.tsx` and `remote-frontend/src/App.tsx`.
- Create placeholder pages for `/nodes` and `/tasks` (inline in AppRouter or as separate lazy files — record the choice).
- Create a `/login` OAuth provider picker page + `/oauth/callback` page (net-new hive shell pages).

## STOP triggers
- If the existing `InvitationPage`/`InvitationCompletePage` import from `./api` in a way that breaks after task 102's re-export refactor — STOP; the re-export in task 102 must keep `getInvitation`/`acceptInvitation` importable from `./api`. Fix task 102's re-export first.
- If `@tanstack/react-query` is not in `remote-frontend/package.json` — STOP and add it (record in ledger).

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/AppRouter.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/AppRouter.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 105` exits 0