---
id: "203"
phase: 2
title: "Rehost: mount Nodes page at /nodes in the hive AppRouter"
status: done
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - frontend/src/pages/Nodes.tsx
  - remote-frontend/src/pages/Nodes.tsx
  - remote-frontend/src/AppRouter.tsx
  - remote-frontend/src/lib/api/organizations.ts
  - remote-frontend/src/hooks/useOrganizations.ts
  - remote-frontend/src/lib/api/index.ts
  - remote-frontend/src/pages/Nodes.test.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Nodes.test.tsx"
allowed_change: create
covers_criteria: [SC1]
---
## Failing test (write first)
File: `remote-frontend/src/pages/Nodes.test.tsx`

Renders `Nodes` page with mocked `QueryClientProvider` (mock `nodesApi.list` → return array of nodes) + `ProfileProvider` (mocked profile with a valid `user_id` + org context). Asserts:
1. The node list renders (at least one NodeCard).
2. The swarm management tabs/sections render (health, projects, labels, templates — whatever the page exposes).

## Change
- **File:** `remote-frontend/src/pages/Nodes.tsx` (CREATE — copy + adapt)
  - **Before:** (file does not exist; the AppRouter placeholder from task 105 renders `<div>Nodes (coming in phase 2)</div>`).
  - **After:** Copy `frontend/src/pages/Nodes.tsx` and adapt:
    - Keep the `@tanstack/react-query` `useQuery` + `nodesApi.list(orgId!)` pattern.
    - The `useUserOrganizations()` hook: if it comes from the node frontend's auth/config context, adapt it to the hive `ProfileProvider` — the hive may not have a multi-org selector yet. If `useUserOrganizations` returns the user's orgs from `UserSystemInfo.profiles`, the hive needs an equivalent. Either: (a) port a `useOrganizations` hook that fetches `/api/organizations` (check if the hive serves this), or (b) hardcode a single-org assumption for now (the hive is single-org in this phase). Record the choice in the ledger.
    - **Sibling alignment:** Read `frontend/src/pages/Nodes.tsx`. List every import. Confirm each resolves under the task 201 aliases + the task 202 copies. Justify divergences (org context source, hooks) in the ledger.

- **File:** `remote-frontend/src/AppRouter.tsx` (EDIT)
  - **Anchor:** the `/nodes` route from task 105 (currently a placeholder).
  - **Before:** `{ path: '/nodes', element: <div>Nodes (coming in phase 2)</div> }` (or lazy equivalent).
  - **After:** `{ path: '/nodes', element: <Suspense><Nodes /></Suspense> }` with `Nodes` lazy-imported from `@/pages/Nodes`. Wrap in `NormalLayout` (already done in task 105 if the route was inside the layout).

## Allowed moves
- Create `remote-frontend/src/pages/Nodes.tsx` + test.
- Edit `remote-frontend/src/AppRouter.tsx` to swap the placeholder.
- Port `useUserOrganizations` or an equivalent if needed (record in ledger).

## STOP triggers
- If `frontend/src/pages/Nodes.tsx` imports `useUserSystem` or `useUserOrganizations` from a node-frontend context that the hive doesn't have — STOP; port the hook or adapt. Record the resolution in the ledger.
- If the hive has no `/api/organizations` endpoint and the page requires multi-org — STOP; record and escalate or hardcode single-org for this phase.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/pages/Nodes.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Nodes.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 203` exits 0
