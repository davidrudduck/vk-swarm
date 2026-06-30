---
topic: ui-overhaul
spec: docs/superpowers/specs/2026-06-28-ui-overhaul.md
status: ready
---

# Plan — ui-overhaul (Midnight Terminal design system)

## Approach

Four phases, each consuming the layer before it. **Phase 1** lands the token foundation in
`index.css` + `tailwind.config.js` — and, per scope decision **C** (D10), makes the brand palette
actually render by promoting the `--vks-*` primitives to `:root` and merging the `.vks-theme`
remap into the live `.dark` selector. Nothing visual works until this lands, so every later task
depends (transitively) on it. **Phase 2** switches components from hardcoded Tailwind colours to
the new tokens and corrects typography/geometry (the A-class bugs ship here). **Phase 3** adds the
new surfaces (Nodes view + route, navbar tab row, task-detail chrome) and the two new presentational
components (`ThemeToggle`, `StatusBadge`, `NodeCard`). **Phase 4** is a manual verification gate
(no code) covering the SC17 smoke-test, WCAG (SC19), persistence (SC20), and the static CI gates.

Tasks within a phase that touch the **same file** are chained via `depends_on` (the executor commits
each task separately; a chain avoids mid-stream conflicts). Independent-file tasks have no dep.

This is a **frontend-only** workstream (D4): no Rust, no API, no generated-type changes. The
backend CI gates (SC1, SC2) are unaffected and verified in Phase 4. Task gates run frontend
typecheck + scoped vitest via inline `WAI_TYPECHECK_CMD`/`WAI_TEST_CMD`; visual/CSS tasks that a
unit test cannot cheaply assert use a `## Manual verification` section with greppable assertions.

## Shared-interface contracts (pin BEFORE authoring — prevents cross-task drift)

These names are FIXED. A consumer task must use exactly these; a producer task must export exactly these.

- **Status token names** (1.002 → consumed by 2.001, 3.002, 3.006): CSS custom properties
  `--status-todo`, `--status-inprogress`, `--status-inreview`, `--status-done`, `--status-cancelled`.
  Values are **bare HSL channel triplets**, so they MUST be consumed wrapped in `hsl(...)`:
  `hsl(var(--status-<key>))` (a bare `var(--status-done)` renders no colour). `<key>` is the
  `TaskStatus` enum value (`todo|inprogress|inreview|done|cancelled`). TaskCard's strip uses the
  `before:` pseudo, i.e. `before:bg-[hsl(var(--status-done))]`.
- **`ThemeToggle`** (2.008 → consumed by 3.003): default export `ThemeToggle` from
  `frontend/src/components/ThemeToggle.tsx`. Props: `{ className?: string }`. No required props.
- **`StatusBadge`** (3.002 → consumed by 3.008): named export `StatusBadge` from
  `frontend/src/components/common/StatusBadge.tsx`. Props:
  `{ status: TaskStatus; showLabel?: boolean; className?: string }`. Renders an 8px dot coloured
  `bg-[hsl(var(--status-<status>))]`; when `showLabel`, appends the localized status label.
- **`NodeCard`** (3.006 → consumed by 3.007): named export `NodeCard` from
  `frontend/src/components/swarm/NodeCard.tsx`. Props: `{ node: NodeForCard; className?: string }`
  where `NodeForCard = Node` (from `@/types/nodes`) — the global node element type, **not** the
  task-scoped `useAvailableNodes`/`ListProjectNodesResponse` (that hook requires a taskId and cannot
  back a global list). Data source for the `/nodes` page is `nodesApi.list(organizationId)` →
  `Node[]` (the same global source `NodeProjectsSection` uses). `NodeCard` reads `node.name` and
  `node.status` (`'pending'|'online'|'offline'|'busy'|'draining'`; online = `status === 'online'`).
  The page resolves an org via `useUserOrganizations()` (ledger-documented limitation: org-scoped,
  not cross-org — a true global view would need a new backend endpoint, out of scope per D4).
- **`lastVisitedProjectId`** (3.004): `localStorage` key string literal `'lastVisitedProjectId'`.

## Phases

1. **phase-1-tokens** — token foundation (`index.css`, `tailwind.config.js`). Brand cascade live (C).
2. **phase-2-components** — component fidelity; token consumption; A-class bugs.
3. **phase-3-surfaces** — new surfaces, components, routes, navbar, task-detail chrome.
4. **phase-4-verification** — manual smoke-test + static gates (no code change).

## Tasks

| id | phase | title | dep: | conflicts: | covers |
|---|---|---|---|---|---|
| 001 | 1 | Promote --vks-* to :root; merge brand remap into .dark; create .light | dep: - | conflicts: 002,003 | SC22 |
| 002 | 1 | Add --status-*, semantic aliases, --strip-width tokens | dep: 001 | conflicts: 001,003 | SC6,SC7 |
| 003 | 1 | Add shadow/glow tokens, vks-pulse keyframe, ANSI texture classes | dep: 002 | conflicts: 001,002 | SC15 |
| 004 | 1 | Add `wordmark` fontFamily key to tailwind.config.js | dep: - | conflicts: none | - |
| 005 | 2 | TaskCard + AllProjectsTaskCard status strips → --status-* tokens | dep: 002 | conflicts: 006 | SC5a,SC5b |
| 006 | 2 | TaskCard typography/meta fidelity + TaskCardHeader title + border-strong hover (SC7 consumer) | dep: 002,005 | conflicts: 005 | - |
| 007 | 2 | TaskCountPills inprogress/inreview colour swap | dep: - | conflicts: none | SC8 |
| 008 | 2 | DaysInColumnBadge flat variant + literal {n}d (no 7d+ cap) | dep: - | conflicts: none | - |
| 009 | 2 | LabelBadge outline variant | dep: - | conflicts: none | - |
| 010 | 2 | Kanban card-state fixes (ring, add-button, status dot, count bg) | dep: - | conflicts: 013 | - |
| 011 | 2 | VKSLogo → font-wordmark | dep: 004 | conflicts: none | - |
| 012 | 2 | Create ThemeToggle component | dep: - | conflicts: none | SC16 |
| 013 | 3 | Kanban grid minmax(264px,1fr) + empty-state ANSI block | dep: 010,003 | conflicts: 010 | SC9,SC10 |
| 014 | 3 | Create StatusBadge component | dep: 002 | conflicts: none | - |
| 015 | 3 | Navbar chrome: Logo→VKSLogo, +Task button, ThemeToggle | dep: 011,012 | conflicts: 016 | SC12,SC13,SC16 |
| 016 | 3 | Navbar second tab row (Board/Nodes/Processes) + lastVisitedProjectId | dep: 015,017 | conflicts: 015 | SC11 |
| 017 | 3 | Nodes page + /nodes route | dep: 018 | conflicts: none | SC14,SC21 |
| 018 | 3 | Create NodeCard component | dep: 002,003 | conflicts: none | SC14 |
| 019 | 3 | ProjectSwitcher folder icon + SearchBar width | dep: - | conflicts: none | - |
| 020 | 3 | Task-detail header+tabs (AttemptHeaderActions: Tabs, StatusBadge, node+labels badges) | dep: 009,014 | conflicts: none | SC18 |
| 021 | 3 | Task-detail footer (GitOperations: Merge/Rebase/Open-in-IDE) | dep: - | conflicts: none | SC18 |
| 022 | 4 | Final manual verification + static CI gates | dep: 005,006,007,008,009,013,015,016,017,019,020,021 | conflicts: none | SC1,SC2,SC3,SC4,SC17,SC19,SC20 |

> Note on SC14/SC18 multi-task coverage: SC14 is claimed by both 017 (route/page) and 018
> (component); SC18 by both 020 (header/tabs) and 021 (footer). SC16 by 012 (create) and 015
> (placement). SC15 by 003 (keyframe) and 018 (application). SC20's write-path lands in 012/015;
> 022 verifies persistence at runtime. SC5a/SC5b both claimed by 005.
