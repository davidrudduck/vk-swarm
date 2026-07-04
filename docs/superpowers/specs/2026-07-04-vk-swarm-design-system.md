---
doc_type: spec
status: active
workstream: vk-swarm-design-system
change_kind: behaviour
---

# vk-swarm-design-system — Midnight Terminal component vocabulary + hive app UI kit

> **Origin:** Claude Design handoff bundle, preserved verbatim at
> `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/`. Design spec at
> `design-spec.md`; gap analysis at `gap-analysis.md` (31 findings: 1 A, 7 B, 23 C). Ingested via
> `/wai:design-handoff-ingest`.
>
> **Relationship to `vk-swarm-hive-ui`:** that workstream's phase 1 (tasks 100-106 — hive auth shell:
> ProfileProvider, oauthApi, useAuth, NormalLayout, AppRouter, root providers) is **reusable
> infrastructure** for this workstream's hive app UI kit. `vk-swarm-hive-ui` phase 2-3 (tasks 202-308,
> the verbatim shadcn-copy approach) is **superseded** — the design system replaces "copy shadcn
> swarm components verbatim" with a purpose-built `.vks-*` component vocabulary.

## Intent (what / why)

The VK-Swarm product has a design language — "Midnight Terminal" — captured in a Claude Design
handoff bundle. The design specifies a complete `.vks-*` component-class vocabulary (buttons,
badges, cards, inputs, switches, checkboxes, tabs, selects, loaders, task cards, node cards,
status badges, settings rows/sections), a refined token set (typography scale, spacing, radius,
motion, glows, textures), and a hive app UI kit (BoardView, Chrome, Panels/TaskDrawer).

**Today:**
- The **node frontend** (`frontend/`) already implements the brand palette (`--vks-*` color tokens
  at `frontend/src/styles/index.css:64-76`), two texture utilities (`.vks-ansi-dither`,
  `.vks-scanlines` at `frontend/src/styles/index.css:9-12`), and feature components (kanban,
  TaskCard, StatusBadge, NodeCard) — but built on **shadcn/ui primitives + Tailwind utility
  classes**, NOT the `.vks-*` component classes the design specifies. The node frontend is the HA
  fallback and is NOT modified by this workstream.
- The **hive frontend** (`remote-frontend/`) has only the auth shell (tasks 100-106 from
  `vk-swarm-hive-ui`) — none of the design system, none of the app UI kit.

This workstream builds the design system in `remote-frontend/` and mounts the hive app UI kit on
top of the existing auth shell. The node frontend is untouched (HA fallback).

## Users / who is affected

- **Operator/admin (browser):** the human who manages nodes, swarm projects, labels, templates,
  and views the cross-node task board. After this workstream they use a hive-hosted console with
  the Midnight Terminal design language — distinct from today's node-frontend shadcn UI.
- **End-user / coder (browser):** the human running coding agents on tasks. After this workstream
  they view tasks/attempts/executions across all nodes in one hive-hosted board (BoardView) with
  the TaskDrawer for per-task detail (diff/logs/attempts panels).
- **Node-frontend users:** unaffected. The node frontend keeps its shadcn/Tailwind UI as the
  always-on fallback during hive outage.

## User stories

- **US1:** As an operator, when I open the hive console, I expect to see a Midnight Terminal
  interface — dark void background, cyan primary actions, terminal-textured empty states — not
  the generic shadcn UI.
- **US2:** As an operator/end-user, when I open the hive console, I expect to see a 5-column
  kanban board (todo/inprogress/inreview/done/cancelled) with tasks from all connected nodes,
  each card showing status, node, labels, and attempt indicator.
- **US3:** As an end-user, when I click a task card, I expect a slide-in drawer (capped
  `min(460px, 90vw)`) with tabs for diff, logs, and attempts — console-styled panels with
  ANSI/BBS line coloring.
- **US4:** As an operator, when I view the nodes page, I expect a NodeCard grid
  (`repeat(auto-fill, minmax(320px, 1fr))`) with per-OS glyphs (mac/linux/windows) and online pulse
  indicators.
- **US5:** As an operator, when I sign into the hive console, I expect the existing auth shell
  (from `vk-swarm-hive-ui` phase 1) wrapped in the Midnight Terminal chrome (Navbar with logo,
  theme toggle, nav tabs).
- **US6:** As an operator, during a hive outage, I expect the node frontend's existing UI to keep
  working as the HA fallback — the design system lands in the hive only, the node frontend is
  untouched.

## Success criteria

- **SC1:** The `.vks-*` component-class vocabulary is implemented in `remote-frontend/` — all
  classes from `design-source/tokens/components.css` (`.vks-btn`, `.vks-badge`, `.vks-card`,
  `.vks-input`, `.vks-switch`, `.vks-checkbox`, `.vks-tabs`, `.vks-select`, `.vks-loader`,
  `.vks-task`, `.vks-node`, `.vks-status`, `.vks-settings`, `.vks-field`, `.vks-alert`,
  `.vks-savebar`) with their hover/focus/active/disabled states. → US1
- **SC2:** The refined token set is implemented in `remote-frontend/` — typography scale
  (`--vks-text-xs`..`--vks-text-5xl`, base=14px), spacing (`--vks-space-1`..`--vks-space-16`, 4px
  grid), radius, motion keyframes (`vks-spin .7s`, `vks-pulse 2s`, switch thumb cubic-bezier),
  shadows, glows — per `design-source/tokens/{typography,spacing,base,components}.css`. → US1
- **SC3:** The texture utilities are implemented in `remote-frontend/` — `.vks-ansi-dither`,
  `.vks-ansi-weave`, `.vks-ansi-grid`, `.vks-scanlines`, `.vks-diagonal-lines`, `.vks-wordmark`,
  `.vks-eyebrow`, `.vks-dashed` per `design-source/tokens/base.css:40-134`. → US1
- **SC4:** Core React components are implemented in `remote-frontend/src/components/core/` —
  Button, Badge, Card, Input, Switch, Checkbox, Tabs, Select, Loader — each matching its
  `design-source/components/core/*.jsx` anatomy, variants, and props. → US1
- **SC5:** Board components are implemented in `remote-frontend/src/components/board/` — TaskCard
  (with 4px status strip + AttemptIndicator), NodeCard (with OS glyph + online pulse), StatusBadge
  (5 statuses) — each matching its `design-source/components/board/*.jsx` anatomy. → US2, US4
- **SC6:** Settings components are implemented in
  `remote-frontend/src/components/settings/` — SettingsSection, SettingsRow (stacked/inline/nested
  variants) — per `design-source/components/settings/*.jsx`. → US1
- **SC7:** The hive app UI kit is implemented in `remote-frontend/src/ui/` — BoardView (5-column
  kanban, ColumnHeader, horizontal scroll, `░▒ no tasks ▒░` empty state), Chrome (Navbar with logo,
  ThemeToggle, NavIcon, NavTab, useBreakpoint), Panels (NodesView NodeCard grid, ProcessesView,
  TaskDrawer with DiffPanel/LogsPanel/AttemptsPanel) — per
  `design-source/ui_kits/vk-swarm-app/*.jsx`. → US2, US3, US4, US5
- **SC8:** The hive app shell (from `vk-swarm-hive-ui` phase 1, tasks 100-106) renders inside the
  Midnight Terminal chrome — the existing AppRouter/NormalLayout/Navbar is restyled or replaced
  with the design system's Chrome. → US5
- **SC9:** The node frontend (`frontend/`) is **unmodified** — no regression to the HA fallback.
  → US6

## Constraints

- **Keep:** the existing hive auth shell (tasks 100-106: ProfileProvider, oauthApi, useAuth,
  AppRouter, root providers). The design system wraps/replaces the slim NormalLayout/Navbar/
  BottomNav from task 104 with the design's Chrome.
- **No node-frontend modifications:** the node frontend is the HA fallback. The design system
  lands in `remote-frontend/` only. Existing `--vks-*` color tokens in
  `frontend/src/styles/index.css:64-76` are NOT edited (they're the node frontend's own impl).
- **Design source is immutable:** `design-source/` is a preserved snapshot; no downstream step
  edits it. The design spec (`design-spec.md`) is the canonical reference.
- **Bare JSON API contract:** the hive returns bare `Json(...)` (no envelope) — established by
  `vk-swarm-hive-ui` tasks 101-102. New API clients in the design system follow the same pattern
  (`makeRequest` + `if (!response.ok) throw` + `response.json() as T`).
- **Bearer token auth:** `localStorage['access_token']` + `Authorization: Bearer` header —
  established by tasks 101-102. No cookie/session auth.
- **Routes nest under `/v1`:** `crates/remote/src/routes/mod.rs:112-113`. All API paths use the
  `/v1` prefix.
- **Fonts from Google Fonts:** Inter, JetBrains Mono, Source Serif 4, Chivo Mono — per
  `design-source/tokens/fonts.css`. Loaded via `@import` in CSS.
- **dark-first + light opt-in:** the design is dark-first; light mode is opt-in via a `.light`
  class on the root. Both themes must work.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **Node-frontend restyle:** the node frontend keeps its shadcn/Tailwind UI. The `.vks-*` color
  tokens at `frontend/src/styles/index.css:64-76` are NOT edited (they're the node frontend's
  own implementation; this workstream does not touch `frontend/`).
- **B-class fidelity fixes to node frontend** (gap-analysis findings 11, 12, 18, 19, 22, 23): the
  node frontend's NodeCard/StatusBadge/texture/font-token drift from the design is NOT fixed here —
  the node frontend is the HA fallback and is untouched. These are documented in `gap-analysis.md`
  for a future node-frontend restyle workstream.
- **A-class bug fixes** (gap-analysis finding 21: fonts): none — fonts already match.
- **Mobile/responsive beyond the design's spec:** the design specifies breakpoints (mobile <640,
  tablet 640-1023, desktop ≥1024) and hit targets (≥34px, 44px touch). This workstream implements
  those; no additional mobile work.
- **Protocol / data-plane work:** owned by `vk-swarm-hive-redesign` (shipped). This workstream
  consumes published Electric shapes; it does not alter the protocol.
- **Cross-node aggregation logic** (the Electric collections for `node_task_assignments`/
  `node_task_output_logs`/`node_task_progress_events`): that was `vk-swarm-hive-ui` phase 3
  (tasks 300-308). This workstream renders the BoardView/TaskDrawer UI; the data wiring is a
  separate concern (may be a follow-up or folded in — see Approach).
- **Backend changes:** no server changes. All routes the UI needs already exist
  (`/v1/profile`, `/v1/oauth/web/*`, `/v1/nodes`, `/v1/swarm/*`, `/v1/tasks/*`,
  `/v1/api/electric/v1/shape`).

## Approach

Three phases, sequentially dependent:

1. **Tokens + textures (foundation):** port the design tokens (colors, typography, spacing,
   radius, motion, shadows, glows) and texture utilities from `design-source/tokens/` into
   `remote-frontend/src/styles/tokens.css` (or split files). Reconcile with the existing
   `remote-frontend/src/index.css` (which currently has only `@tailwind base/components/utilities`).
   The token phase is the foundation — components and UI kit depend on it.

2. **Core + board + settings components:** implement the React components in
   `remote-frontend/src/components/{core,board,settings}/` matching the `design-source/components/`
   JSX anatomy. Each component is a thin React wrapper over the `.vks-*` CSS classes from phase 1.
   Components are presentational (no data fetching); they take props and render. Tests verify
   structure + variants + states against the design source.

3. **App UI kit + shell integration:** implement BoardView, Chrome, Panels/TaskDrawer in
   `remote-frontend/src/ui/` per `design-source/ui_kits/vk-swarm-app/*.jsx`. Replace the slim
   task-104 NormalLayout/Navbar/BottomNav with the design's Chrome. Mount BoardView at `/tasks`,
   NodesView at `/nodes` (replacing the placeholder), ProcessesView at `/processes`. The TaskDrawer
   renders on task card click. Data wiring (Electric collections + REST clients) is folded in here
   — the BoardView consumes task data; if the Electric collections from `vk-swarm-hive-ui` phase 3
   aren't built yet, BoardView uses REST via `@tanstack/react-query` as a fallback (the data plane
   exists; only the client collections are missing).

Phases compose: phase 1 delivers the token foundation; phase 2 delivers the component vocabulary
that phase 3's UI kit composes into screens.

## Design / architecture

### Token layer (phase 1)

```
remote-frontend/src/styles/
├── index.css              # entry: @import tokens + base
├── tokens.css             # :root + .dark + .light — all --vks-* tokens
├── base.css               # element defaults + texture utilities (.vks-ansi-*, .vks-scanlines, etc.)
└── components.css         # .vks-btn, .vks-card, .vks-task, .vks-node, etc. (all component classes)
```

**Token reconciliation:** the design's `colors.css` uses hex values; the node frontend's existing
`--vks-*` tokens (`frontend/src/styles/index.css:64-76`) use HSL triplets. This workstream uses
the design's hex values directly (via `#hex` or converted to HSL where the CSS architecture
expects HSL — decision deferred to the task). The node frontend's tokens are NOT edited.

**Texture utilities:** ported verbatim from `design-source/tokens/base.css:40-134`. The two
existing utilities in `frontend/src/styles/index.css:9-12` (`.vks-ansi-dither`, `.vks-scanlines`)
are the reference pattern but are NOT edited — the hive gets its own copy.

### Component layer (phase 2)

```
remote-frontend/src/components/
├── core/
│   ├── Button.tsx         # variants: primary/secondary/outline/ghost/destructive/link; sizes: xs/sm/md/lg/icon
│   ├── Badge.tsx           # variants: default/secondary/destructive/outline; optional dot
│   ├── Card.tsx            # compound: Card > Header > {Title, Description} > Content > Footer
│   ├── Input.tsx           # optional mono prop
│   ├── Switch.tsx          # controlled/uncontrolled
│   ├── Checkbox.tsx        # controlled/uncontrolled
│   ├── Tabs.tsx            # segmented control
│   ├── Select.tsx          # native select styled
│   └── Loader.tsx          # spinner sm/md/lg
├── board/
│   ├── TaskCard.tsx        # 4px status strip + AttemptIndicator
│   ├── NodeCard.tsx        # OS glyph (mac/linux/windows) + online pulse
│   └── StatusBadge.tsx     # 5 statuses, dot + optional label
└── settings/
    ├── SettingsSection.tsx # Card with header + stacked body
    └── SettingsRow.tsx     # stacked/inline/nested variants
```

Each component is a thin TS React wrapper over the `.vks-*` CSS classes. Props match the
`design-source/components/*/*.jsx` signatures (typed via the `.d.ts` siblings). No data fetching —
presentational only.

### App UI kit layer (phase 3)

```
remote-frontend/src/ui/
├── BoardView.tsx           # 5-column kanban, ColumnHeader, horizontal scroll, empty state
├── Chrome.tsx              # Navbar (Logo + ThemeToggle + NavIcon + NavTab), useBreakpoint hook
├── NodesView.tsx           # NodeCard grid repeat(auto-fill, minmax(320px, 1fr))
├── ProcessesView.tsx       # loader + status + name + node + duration rows
├── TaskDrawer.tsx          # slide-in min(460px, 90vw), Tabs(diff/logs/attempts)
├── DiffPanel.tsx           # console bg, add/del/ctx/meta line classes
├── LogsPanel.tsx           # console bg, muted/ok/fg/cy/err line classes
└── AttemptsPanel.tsx       # attempt rows
```

**Shell integration:** the existing `remote-frontend/src/components/layout/NormalLayout.tsx` (task
104, 15 lines) + `Navbar.tsx` (67 lines) + `BottomNav.tsx` (68 lines) are replaced by the design's
Chrome. The `AppRouter.tsx` (task 105) keeps its route structure but the layout component changes.
ProfileProvider + oauthApi + useAuth (tasks 101-103) stay — the Chrome's ThemeToggle reads/writes
a `.light`/`.dark` class on `document.documentElement`.

**Data wiring:** BoardView consumes task data. The `vk-swarm-hive-ui` phase 3 (Electric collections)
is superseded by this workstream — BoardView uses `@tanstack/react-query` against the hive's
existing REST routes (`/v1/tasks/bulk`, `/v1/tasks/by-source`) for the initial render, and may
subscribe to the Electric proxy (`/v1/api/electric/v1/shape`) for live updates if the collections
are added. The REST-first approach is the fallback; Electric is the optimization. Decision: REST
first (the routes exist and are tested); Electric collections are a follow-up optimization, not a
blocker for the UI.

### Decisions

- **D1: Design system lands in `remote-frontend/` only, node frontend untouched.** The node
  frontend is the HA fallback; its existing `--vks-*` tokens and shadcn UI stay. This is reversible
  (the node frontend could be restyled later) but is treated as a constraint for this workstream.
  Not irreversible → no ADR.
- **D2: REST-first data wiring, Electric optional.** BoardView uses REST via react-query against
  existing `/v1/tasks/*` routes. Electric collections (the `vk-swarm-hive-ui` phase 3 plan) are a
  follow-up optimization, not a blocker. Reversible → no ADR.
- **D3: Replace task-104 NormalLayout/Navbar/BottomNav with design's Chrome.** The slim shell from
  `vk-swarm-hive-ui` task 104 is replaced, not extended. The auth shell (tasks 101-103, 105-106)
  stays. Reversible (the task-104 files are in git history) → no ADR.
- **D4: Hex token values from design, not HSL triplets from node frontend.** The design's
  `colors.css` uses hex (`#1a1a33`); the node frontend uses HSL triplets (`240 20% 5%`). This
  workstream uses the design's values. The node frontend's tokens are NOT edited. Reversible →
  no ADR.

No irreversible decisions (no deletes, no migrations, no wire-format changes, no breaking
renames). All decisions are reversible in `remote-frontend/`.

## Test strategy

- **Unit tests** per component: render with each variant/prop/state, assert structure (class
  names, element hierarchy) matches the `design-source/components/*/*.jsx` anatomy. Use
  `@testing-library/react` (already installed by task 100).
- **Token tests:** assert each `--vks-*` token is defined in `:root` and resolves to the design's
  value. Compile-time + runtime (CSS is parsed, variables resolve).
- **Texture utility tests:** assert each `.vks-*` utility class produces the expected visual
  property (e.g. `.vks-scanlines::after` has `content: ""`).
- **Integration tests:** BoardView renders 5 columns with ColumnHeader; TaskDrawer opens on card
  click; Chrome renders Navbar with theme toggle; NodesView renders NodeCard grid.
- **Parity tests (SC9):** `cd frontend && npx tsc --noEmit` still passes (node frontend
  unmodified). The node frontend's `npm run lint` and `npm run build` still pass.
- **Visual smoke:** `cd remote-frontend && npm run build` exits 0 (Vite builds the design system +
  UI kit without errors).