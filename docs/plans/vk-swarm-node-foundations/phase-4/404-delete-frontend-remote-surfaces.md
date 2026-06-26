---
id: "404"
phase: 4
title: Delete the frontend Nodes-management feature (page, components, context, dialog, hooks, nav)
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/pages/Nodes.tsx
  - frontend/src/components/nodes/NodeCard.tsx
  - frontend/src/components/nodes/NodeList.tsx
  - frontend/src/components/nodes/NodeDetailPanel.tsx
  - frontend/src/components/nodes/NodeStatusBadge.tsx
  - frontend/src/components/nodes/NodeProjectsList.tsx
  - frontend/src/components/nodes/index.ts
  - frontend/src/components/dialogs/nodes/MergeNodesDialog.tsx
  - frontend/src/contexts/NodesContext.tsx
  - frontend/src/hooks/useNodeMutations.ts
  - frontend/src/hooks/useNodeProjects.ts
  - frontend/src/App.tsx
  - frontend/src/components/layout/Navbar.tsx
irreversible: true
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---
## Failing test (write first)

`N/A — covered by: frontend `tsc --noEmit` + `npm run lint`` (the gate's frontend check). Deleting a
dead UI feature has no unit assertion; correctness = "files gone, no dangling imports, frontend
type-checks and lints clean". Verified by the `## Manual verification` section + the `## Done when`
frontend check.

> **🚧 HUMAN GATE (irreversible):** deletes frontend code (the Nodes feature) we did not author for the
> standalone node. A `reviews/404.approved` token must exist before this runs.

> **SCOPE NOTE — this task is deliberately narrowed; see the discrepancy report.** ADR-0002/§4 name a
> broader "remote surfaces" set (remote badges on cards, `useMergedProjects`, remote stream/diff hooks).
> Verification showed those are **entangled** with live local task/attempt UI:
> `useNodeLogStream`→`ProcessLogsViewer`, `useRemoteConnectionStatus`→`AttemptHeaderActions`,
> `useDiffStream`→`DiffsPanel`/`useDiffSummary`, `useAvailableNodes`→`CreateAttemptDialog`,
> `useMergedProjects`→`ProjectList`/`ProjectSwitcher` (which are typed on `MergedProject`, not
> `Project`). Removing them is a multi-component rewrite, not a clean delete, so it is OUT of this task
> (carved into the `vk-swarm-node-ui-localize` workstream — see report). This task removes ONLY the self-contained Nodes-
> management feature, whose every consumer is internal to the feature or a removable wiring line.

## Change

**A. Delete the Nodes feature files (`git rm`):**
```text
git rm frontend/src/pages/Nodes.tsx
git rm frontend/src/components/nodes/NodeCard.tsx
git rm frontend/src/components/nodes/NodeList.tsx
git rm frontend/src/components/nodes/NodeDetailPanel.tsx
git rm frontend/src/components/nodes/NodeStatusBadge.tsx
git rm frontend/src/components/nodes/NodeProjectsList.tsx
git rm frontend/src/components/nodes/index.ts
git rm frontend/src/components/dialogs/nodes/MergeNodesDialog.tsx
git rm frontend/src/contexts/NodesContext.tsx
git rm frontend/src/hooks/useNodeMutations.ts
git rm frontend/src/hooks/useNodeProjects.ts
```

(Each was verified to have NO consumer outside this deleted set: `components/nodes/*`,
`NodesContext`/`NodesProvider`, `MergeNodesDialog`, `useNodeMutations`, `useNodeProjects` are
referenced only within the Nodes feature itself. `useNode` and `nodesApi` are NOT deleted — they have
live non-Nodes consumers — so they stay.)

**B. `frontend/src/App.tsx` — remove the page import + its two routes.**
- Remove the import (L10): `import { Nodes } from '@/pages/Nodes';`
- Remove the two routes (L155–156):
```text
                    <Route path="/nodes" element={<Nodes />} />
                    <Route path="/nodes/:nodeId" element={<Nodes />} />
```

**C. `frontend/src/components/layout/Navbar.tsx` — remove the nav entry + now-unused icon import.**
- Remove the nav entry (L54): `  { label: 'Nodes', icon: Server, to: '/nodes' },`
- Remove the `Server,` line from the `lucide-react` import block (L13) — verified `Server` is used
  ONLY by that nav entry (2 occurrences: import + entry). If another reference appears, KEEP the import.

## Allowed moves

- `git rm` the 11 listed Nodes-feature files.
- Edit ONLY `App.tsx` (one import + two `<Route>` lines) and `Navbar.tsx` (one nav entry + one icon
  import line).
- Do NOT delete or edit `useNode.ts`, `lib/api/nodes.ts` (`nodesApi`), `components/org/NodeApiKeySection.tsx`,
  `components/swarm/NodeProjectsSection.tsx`, `useNodeLogStream`, `useRemoteConnectionStatus`,
  `useDiffStream`, `useAvailableNodes`, `useMergedProjects`, the task/project cards, `ProcessLogsViewer`,
  `DiffsPanel`, `AttemptHeaderActions`, `CreateAttemptDialog`, or `hooks/index.ts` — those are the
  entangled set, out of scope (follow-ups).
- Backend untouched (this is a `frontend/` task).

## STOP triggers

- `tsc --noEmit` reports a dangling import to any deleted file from a file NOT in this task's `files:`
  (means a consumer outside the verified internal set exists) — STOP; that file belongs to a follow-up,
  do not expand scope.
- `Server` icon turns out to be used elsewhere in Navbar — keep the import, remove only the entry.
- A deleted hook (`useNodeMutations`/`useNodeProjects`) is imported somewhere unexpected — STOP.
- Any required edit falls outside `App.tsx`/`Navbar.tsx`.

## Manual verification (record in decisions-ledger)

1. `cd frontend && npx tsc --noEmit` → exits 0 (no `Cannot find module '@/pages/Nodes'` or similar
   dangling-import errors).
2. `cd frontend && npm run lint` → exits 0.
3. `git grep -nE "pages/Nodes|components/nodes|NodesContext|useNodeMutations|useNodeProjects|MergeNodesDialog" frontend/src`
   → no import/reference hits remain (only the deletions themselves).
4. Record in the decisions-ledger: the Nodes feature is removed; the entangled remote-display set
   (stream/diff hooks, remote card badges, `useMergedProjects` repoint) is carved out to the
   `vk-swarm-node-ui-localize` workstream (a deliberate, user-approved scope split — see ledger).

## Done when

`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="cd frontend && npm run lint" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 404` exits 0

> Frontend task (not Rust): the gate's TS type-check + lint ARE the verification (overridden inline so
> the gate runs the frontend's own toolchain from `frontend/`, per decisions-ledger Trap 1).
