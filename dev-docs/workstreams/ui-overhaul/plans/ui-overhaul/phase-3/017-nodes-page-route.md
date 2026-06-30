---
id: "017"
phase: 3
title: Nodes page (NodeCard grid) + /nodes route
status: ready
depends_on: ["018"]
parallel: false
conflicts_with: []
files:
  - frontend/src/pages/Nodes.tsx
  - frontend/src/App.tsx
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC14, SC21]
---
## Failing test (write first)
N/A — covered by manual verification (scope_test N/A; greppable + browser assertions below).
`WAI_TEST_CMD="true"`.

### Why `allowed_change: mixed`
This task is **one new file** (`frontend/src/pages/Nodes.tsx`, an Addition) **plus one small edit**
(`frontend/src/App.tsx`, adding an import + a `<Route>`). The gate's `create` check accepts only
Additions for the listed files, so App.tsx (a modify) would fail a `create` gate; an `edit` gate
would reject the brand-new file. `mixed` is the schema-valid value for exactly this shape
("structural checks relaxed; leans on the adversarial panel; use sparingly" —
`plugins/wai/schema/task.frontmatter.md`). plan.md lists this as a single task, so it stays one
task with `allowed_change: mixed`. The edit to App.tsx is intentionally minimal (see Allowed moves).

## Change

### Data source (DECISION — same as task 018; record in decisions-ledger)
The global `/nodes` page renders one `NodeCard` per node. `NodeCard`'s `NodeForCard` type is `Node`
(from `@/types/nodes`) — see task 018 for the full rationale (the plan's nominal
`useAvailableNodes`/`ListProjectNodesResponse` source is task-scoped and unusable for a global list;
`nodesApi.list(orgId)` → `Node[]` is the only global source and its `name`/`status` fields match
the contract). **Limitation (ledger):** this source is **org-scoped** — there is no org-free node
list — so the page first resolves an organization (default to the first non-personal org, fallback
first org; no-org → empty state), mirroring `useOrganizationSelection`'s default logic. A truly
cross-org view would need a new backend endpoint = out of scope (D4, UI-only).

Use an inline `useQuery` (do NOT create a new `useNodes` hook file — that would be a third file
outside this task's `files:`), mirroring `NodeProjectsSection`'s call:
`useQuery(['nodes', orgId], () => nodesApi.list(orgId), { enabled: !!orgId, staleTime: 30_000 })`.

### File 1 — create `frontend/src/pages/Nodes.tsx`
- **Export convention:** match `Processes.tsx` — a **named** export (`export function Nodes() {…}`),
  NOT a default export. (Verified: `Processes.tsx` uses `export function Processes()` and App.tsx
  imports it as `import { Processes } from '@/pages/Processes';`.)
- **Org resolution:** `useUserOrganizations()` (from `@/hooks`) returns `ListOrganizationsResponse`
  (`.organizations: OrganizationWithRole[]`). Pick `organizations.find(o => !o.is_personal) ??
  organizations[0]` and use its `.id` as `orgId` (the same default rule as `useOrganizationSelection`).
- **Heading font:** `<h2 className="font-serif text-2xl font-semibold">Nodes</h2>`. `tailwind.config.js`
  exposes both `serif: ['"Source Serif 4"', 'Georgia', 'serif']` and
  `heading: 'var(--font-heading, var(--font-ui))'`. Use **`font-serif`** (the design's display face is
  Source Serif 4; the direct key resolves deterministically without depending on whether the
  `--font-heading` CSS var — set in task 001 — has cascaded into this subtree). Document this choice
  in the ledger.
- **Grid:** `grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3 max-w-[1000px]`.
- **Map:** one `<NodeCard key={node.id} node={node} />` per node, importing
  `import { NodeCard } from '@/components/swarm/NodeCard';`.
- **States (keep simple):** loading → a spinner; empty (no org OR zero nodes) → a muted message;
  otherwise the grid. (No filters, no mutations — this is a read-only overview.)

Suggested contents (adapt only if a STOP trigger forces it):
```tsx
import { useQuery } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { NodeCard } from '@/components/swarm/NodeCard';
import { nodesApi } from '@/lib/api';
import { useUserOrganizations } from '@/hooks';

export function Nodes() {
  const { data: orgData } = useUserOrganizations();
  const organizations = orgData?.organizations ?? [];
  // Same default as useOrganizationSelection: first non-personal, else first.
  const orgId =
    (organizations.find((o) => !o.is_personal) ?? organizations[0])?.id;

  const {
    data: nodes = [],
    isLoading,
  } = useQuery({
    queryKey: ['nodes', orgId],
    queryFn: () => nodesApi.list(orgId!),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  return (
    <div className="space-y-6 p-8 pb-16 md:pb-8 h-full overflow-auto">
      <h2 className="font-serif text-2xl font-semibold">Nodes</h2>

      {!orgId || isLoading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : nodes.length === 0 ? (
        <p className="text-muted-foreground">No nodes connected yet.</p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3 max-w-[1000px]">
          {nodes.map((node) => (
            <NodeCard key={node.id} node={node} />
          ))}
        </div>
      )}
    </div>
  );
}
```

### File 2 — edit `frontend/src/App.tsx` (minimal)
- **Import** (match the `Processes` import style, near line 10 — a static, non-lazy named import):
  add `import { Nodes } from '@/pages/Nodes';` immediately after the existing
  `import { Processes } from '@/pages/Processes';` line.
- **Route** (inside `NormalLayout`, adjacent to the `/processes` route ~line 154): add
  `<Route path="/nodes" element={<Nodes />} />` directly after the existing
  `<Route path="/processes" element={<Processes />} />` line.

## Allowed moves
- Create `frontend/src/pages/Nodes.tsx` as the named-export page above.
- In `App.tsx`: ONLY add the one `import { Nodes } …` line and the one `<Route path="/nodes" …>`
  line. Do not touch any other import, route, layout, or the `FullAttemptLogsPage` route.

## STOP triggers
- **No usable node data source** — if `nodesApi.list` / `Node[]` no longer exists, or `@/types/nodes`
  no longer exports a `Node` with `id`/`name`/`status` (halt + report; do NOT fabricate a backend
  endpoint — this is a UI-only workstream, D4).
- `frontend/src/pages/Nodes.tsx` already exists (halt; reconcile, don't overwrite).
- A `/nodes` route or a `import … '@/pages/Nodes'` already exists in `App.tsx` (halt; reconcile).
- Task 018's `NodeCard` (named export) / its `NodeForCard` type is absent in
  `frontend/src/components/swarm/NodeCard.tsx` (halt — 018 not applied; this task depends on it).
- `Processes` is no longer a **named** export in `@/pages/Processes` (halt — re-derive the page
  export convention before pinning `export function Nodes`).

## Manual verification (record in decisions-ledger)
- `ls frontend/src/components/swarm/NodeCard.tsx` → exists (018 landed).
- `grep -F 'path="/nodes"' frontend/src/App.tsx` → match (SC21).
- `grep -F "import { Nodes } from '@/pages/Nodes'" frontend/src/App.tsx` → match.
- `grep -E 'export function Nodes' frontend/src/pages/Nodes.tsx` → match (named export, matches `Processes`).
- `cd frontend && npx tsc --noEmit` → passes.
- Manual browser: navigate to `/nodes` — the page renders `NodeCard`s (or the empty state when no
  org / no nodes) (SC14).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 017` exits 0
