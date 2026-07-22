---
id: "018"
phase: 3
title: Create NodeCard component (OS-glyph row, mono name, online pulse dot)
status: passed
depends_on: ["002", "003"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/swarm/NodeCard.tsx
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC14]
---
## Failing test (write first)
N/A — covered by manual verification (scope_test N/A; greppable assertions below). A vitest is not
cheap here and adds no coverage a `tsc` + `grep` gate doesn't already give. `WAI_TEST_CMD="true"`.

## Change

NEW component per the plan's Shared-interface contract: **named export `NodeCard`** from
`frontend/src/components/swarm/NodeCard.tsx`, props `{ node: NodeForCard; className?: string }`.

### Data source / `NodeForCard` type (DECISION — record in decisions-ledger)
plan.md's contract names `NodeForCard` as the element type of `useAvailableNodes`
(`ListProjectNodesResponse` → `ProjectNodeInfo`). That source is **task-scoped** (it requires a
`taskId`) and so cannot feed a **global** `/nodes` page (task 017). The only existing *global*
node list is `nodesApi.list(organizationId)` → `Node[]` (from `@/types/nodes`) — the exact source
`NodeProjectsSection` already renders. Decisively: plan.md's own contract says the card reads
**`node.name`** and a status field with values **`'online'|'offline'|...`** — those are `Node`'s
field names (`name`, `status: NodeStatus`), NOT `ProjectNodeInfo`'s (`node_name`, `node_status`).
So `Node` is both the only viable global element type AND the one that makes plan.md's stated
field-access correct. **`NodeForCard = Node`**, single-sourced here via
`export type NodeForCard = Node;` and imported by task 017.

- **Online-ness:** `node.status === 'online'` (`NodeStatus = 'pending'|'online'|'offline'|'busy'|'draining'`).
- **Limitation (ledger):** `Node` has no live agent-count field, and the global source is
  org-scoped (no cross-org endpoint exists). A truly cross-org / agent-count-bearing card would
  need a new backend endpoint = out of scope for this UI-only workstream (D4). The right slot
  therefore degrades to an offline badge (offline) or a status label (online) — see Anatomy.

### SIBLING-ALIGN (rubric clause 9)
Sibling read (NOT edited; not in `files:`): `frontend/src/components/swarm/NodeProjectsSection.tsx`.
How the sibling renders a node today (the row this card replaces/parallels):
- A `<Monitor className="h-4 w-4 text-muted-foreground shrink-0" />` glyph (~line 323).
- The node name in `<span className="font-medium truncate">{node.name}</span>` (~line 324–326).
- Online state via a shadcn `<Badge variant={node.status === 'online' ? 'default' : 'secondary'}>`
  showing the raw `node.status` text (~line 327–334).

Conventions mirrored: same `<Monitor>` glyph (single icon for every node — no OS-specific
iconography), same `node.name` source, same `node.status === 'online'` online test, and `className`
merged last via `cn(...)` from `@/lib/utils`.
Justified divergences (per spec Phase 3 anatomy — this is the *card* surface, not the settings row):
- The name is rendered `font-mono` (terminal aesthetic) rather than `font-medium`.
- Online state is shown as a pulsing dot (the `vks-pulse` keyframe from task 003), not a text badge.
- The glyph sits in a 36×36 raised-surface container instead of a bare inline icon.
- OS-glyph-from-`capabilities.os` is deliberately deferred (note in-task as future work) — the
  sibling uses one `<Monitor>` for all nodes and inventing OS iconography is overreach here.

### Anatomy (spec Phase 3)
A single row inside a card surface:
- **OS-glyph container:** 36×36px, `rounded-md`, raised surface background via
  `bg-[hsl(var(--surface-raised))]` (token from task 002), centring a `<Monitor>` glyph.
- **Node name:** `font-mono`, truncating, fed by `node.name`.
- **Online indicator:** when `node.status === 'online'`, an 8×8px pulse dot
  `bg-[hsl(var(--status-done))]` with class `animate-[vks-pulse_2s_ease-in-out_infinite]` (keyframe
  from task 003 — SC15 application). When NOT online, a dim static dot
  `bg-[hsl(var(--vks-text-dim))]` with **no** animation.
- **Right slot:** when offline, an offline `<Badge variant="secondary">` (or equivalent muted
  label); when online, a muted status label. No fabricated agent-count.

**CRITICAL — no template-literal class names.** Tailwind only emits an arbitrary-value class when it
sees the *complete literal string* in source. Every `bg-[hsl(var(--…))]` and the
`animate-[vks-pulse_2s_ease-in-out_infinite]` class MUST be written as static literals (no
`bg-[hsl(var(--status-${x}))]`), or the colour/animation renders as nothing.

### Full file contents (allowed_change: create)
```tsx
import { Monitor } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { Node } from '@/types/nodes';

/**
 * The per-node element type the Nodes page (task 017) maps over.
 *
 * plan.md's contract nominally points at `useAvailableNodes`
 * (`ListProjectNodesResponse`), but that source is task-scoped and cannot feed
 * a global /nodes page. The only global node list is `nodesApi.list(orgId)` →
 * `Node[]` (the source NodeProjectsSection uses), whose `name` / `status` fields
 * match plan.md's stated `node.name` + `'online'|'offline'` access. So
 * NodeForCard === Node. (Limitation: org-scoped, no agent-count — see task file.)
 */
export type NodeForCard = Node;

interface NodeCardProps {
  node: NodeForCard;
  className?: string;
}

/**
 * Presentational card for one swarm node: a raised OS-glyph tile, the node name
 * in mono, and an online pulse dot (vks-pulse keyframe from task 003) or a dim
 * offline dot. Mirrors NodeProjectsSection's Monitor glyph + node.name + the
 * `node.status === 'online'` online test; restyled for the card surface.
 */
export function NodeCard({ node, className }: NodeCardProps) {
  const isOnline = node.status === 'online';

  return (
    <div
      className={cn(
        'flex items-center gap-3 rounded-md border border-border bg-[hsl(var(--surface-card))] p-3',
        className
      )}
    >
      {/* OS-glyph tile — 36x36 raised surface. (Future: OS-specific glyph from
          node.capabilities.os; sibling uses a single Monitor for all nodes.) */}
      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-[hsl(var(--surface-raised))]">
        <Monitor className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
      </div>

      {/* Online/offline dot — pulse only when online (SC15 application). */}
      {isOnline ? (
        <span
          className="h-2 w-2 shrink-0 rounded-full bg-[hsl(var(--status-done))] animate-[vks-pulse_2s_ease-in-out_infinite]"
          aria-hidden="true"
        />
      ) : (
        <span
          className="h-2 w-2 shrink-0 rounded-full bg-[hsl(var(--vks-text-dim))]"
          aria-hidden="true"
        />
      )}

      {/* Node name — mono, truncating. */}
      <span className="min-w-0 flex-1 truncate font-mono text-sm">
        {node.name}
      </span>

      {/* Right slot — offline badge, or a muted status label when online. No
          fabricated agent count (Node carries none). */}
      {isOnline ? (
        <span className="shrink-0 text-xs text-muted-foreground">
          {node.status}
        </span>
      ) : (
        <Badge variant="secondary" className="shrink-0">
          {node.status}
        </Badge>
      )}
    </div>
  );
}
```

## Allowed moves
- ONLY: create `frontend/src/components/swarm/NodeCard.tsx` with the contents above. No edits to
  `NodeProjectsSection` (sibling read only) or any consumer (the page + route is task 017).

## STOP triggers
- `frontend/src/components/swarm/NodeCard.tsx` already exists (halt; reconcile, don't overwrite).
- The `Node` type in `@/types/nodes` no longer exposes `name: string` / `status: NodeStatus`
  (halt — `NodeForCard` mapping would be wrong; re-derive from the current type and the chosen
  global source).
- The `vks-pulse` keyframe from task 003 is absent in `index.css`
  (`grep 'vks-pulse' frontend/src/styles/index.css` → no match → halt; 003 not applied — the pulse
  class would be inert).
- The `--surface-raised` / `--surface-card` / `--status-done` tokens from task 002 are absent
  (halt — 002 not applied; the tile/dot would render colourless).

## Manual verification (record in decisions-ledger)
- `cd frontend && npx tsc --noEmit` → passes.
- `grep -E 'export function NodeCard|export const NodeCard' frontend/src/components/swarm/NodeCard.tsx` → match (SC14).
- `grep 'vks-pulse' frontend/src/components/swarm/NodeCard.tsx` → match (SC15 application).
- `grep -F 'export type NodeForCard = Node' frontend/src/components/swarm/NodeCard.tsx` → match
  (the type task 017 imports).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 018` exits 0
