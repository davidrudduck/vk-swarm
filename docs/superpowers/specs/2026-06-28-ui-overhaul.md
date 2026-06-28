---
doc_type: spec
status: active
workstream: ui-overhaul
change_kind: behaviour
---

# ui-overhaul — Implement the "Midnight Terminal" VK-Swarm design system

> **Design source:** `dev-docs/designs/2026-06-28-ui-overhaul/design-source/` (verbatim snapshot —
> do not modify). **Canonical spec:** `dev-docs/designs/2026-06-28-ui-overhaul/design-spec.md`.
> **Gap analysis:** `dev-docs/designs/2026-06-28-ui-overhaul/gap-analysis.md` — 57 verified
> findings classified A/B/C.
>
> **Design-source translation note:** the design-source uses `[data-theme="light"]` attribute
> selectors and hex colour values. The product uses class-based `.dark`/`.light` selectors and
> HSL channel triplets (`240 20% 5%`) for Tailwind's `hsl(var(--...))` consumption pattern.
> Translate all design-source hex values to HSL channel triplets, and `[data-theme="light"]`
> selectors to `.light { }`, when porting tokens.

## Intent (what / why)

A professional design system ("Midnight Terminal") was authored in claude.ai/design for VK-Swarm
and preserved as a handoff bundle. The current frontend was built organically and diverges from that
system in colour tokens, component anatomy, interactive states, typography, layout geometry, and
missing surface components (Nodes view, slide-from-right task drawer, empty-state textures).

This workstream aligns the product UI to the design spec so that:

1. All colour tokens are expressed as CSS custom properties derived from the `--vks-*` brand palette
   — no hardcoded Tailwind utility colours (`bg-green-500`, `bg-red-500`, etc.) remaining in
   components that should use the token system.
2. Task card anatomy, status strips, attempt indicators, meta row, and selection/hover states match
   the spec exactly.
3. The kanban board columns use the correct minimum width (264 px), empty-state texture, and header
   geometry.
4. The navbar renders the `VKSLogo` wordmark (Chivo Mono, "VK" cyan, "-SWARM" foreground), a
   primary "+ Task" button, and a second tab row for Board / Nodes / Processes views.
5. A `NodeCard` component exists and is used in a Nodes view rendered on the Nodes tab (`/nodes`
   global route, consistent with the existing `/processes` global route).
6. The task detail surface keeps its resizable multi-pane workbench (diff / terminal / files /
   processes), with the in-panel chrome aligned to the drawer spec — see ADR-0006
   (`dev-docs/adr/0006-task-detail-retain-workbench-over-drawer.md`).
7. Light and dark themes are fully token-driven. "Midnight Terminal" maps to the **existing
   `ThemeMode.DARK` path** (`ThemeProvider.tsx` applies the `.dark` class to `<html>`) — no new
   enum variant, no backend change. A `ThemeToggle` cycles dark ↔ light using the existing
   `setTheme` / `updateAndSaveConfig` mechanism.
8. No console/linting errors introduced. All CI checks pass.

## Users / who is affected

- All VK-Swarm users — visual appearance of every primary surface changes.
- Developers — component API contracts may change for `TaskCard`, `Badge`, `Button`, `NodeCard`.

## Success criteria

Testable definitions of "done":

- SC1: `cargo clippy --all --all-targets --all-features -- -D warnings` passes.
- SC2: `cargo test --workspace` passes.
- SC3: `cd frontend && npm run lint` passes (zero ESLint errors).
- SC4: `cd frontend && npx tsc --noEmit` passes.
- SC5a: Hardcoded-colour **removal** — `grep -r 'bg-green-500\|bg-red-500\|bg-amber-500\|bg-blue-500' frontend/src/components/tasks/ frontend/src/components/projects/TaskCountPills.tsx` → zero matches.
- SC5b: Hardcoded-colour **replacement** — `grep -r 'var(--status-' frontend/src/components/tasks/TaskCard.tsx frontend/src/components/tasks/AllProjectsTaskCard.tsx` → at least one match per file (confirms token replacement, not just class removal).
- SC6: `--status-todo`, `--status-inprogress`, `--status-inreview`, `--status-done`, `--status-cancelled`
  are defined under the `.dark { }` selector in `frontend/src/styles/index.css` (not `.vks-theme`),
  with light overrides under `.light { }`. TaskCard status strips consume them via `var(--status-*)`.
- SC7: `--border-strong`, `--surface-card`, `--surface-raised` are defined under `.dark { }` in
  `index.css` and consumed (referenced by at least one component).
- SC8: `TaskCountPills` (`frontend/src/components/projects/TaskCountPills.tsx`) — inprogress colour →
  blue (`#3b82f6`), inreview → amber (`#ffb800`). Was swapped — A-class bug.
- SC9: Kanban column min-width: `minmax(264px, 1fr)` in
  `frontend/src/components/ui/shadcn-io/kanban/index.tsx` (was `minmax(200px,400px)`).
- SC10: Empty column renders a `.vks-ansi-dither.vks-scanlines` block containing `░▒ no tasks ▒░`
  centered text in `font-mono text-xs text-muted-foreground` (not blank).
- SC11: Navbar second row has Board / Nodes / Processes tabs; active tab has 2px cyan bottom border.
- SC12: `<VKSLogo` rendered in `frontend/src/components/layout/Navbar.tsx` (not SVG `<Logo>`).
- SC13: `+ Task` button in Navbar: `variant="default" size="sm"` with text label.
- SC14: `NodeCard` component exists at `frontend/src/components/swarm/NodeCard.tsx`; `/nodes` route
  exists in `frontend/src/App.tsx`; navigating to `/nodes` renders `NodeCard` components.
- SC15: `@keyframes vks-pulse` defined in `index.css` with: animated property `opacity` and/or
  `box-shadow`, easing `ease-in-out`, iteration-count `infinite`. Applied to the online NodeCard
  status dot. Does not reference the undefined token `--vks-emerald-hsl` — use `--status-done`
  or its literal HSL value `152 100% 50%` instead.
- SC16: A theme toggle (sun/moon icon) is present in the Navbar.
- SC17: Manual browser smoke-test checklist (all items pass, zero console errors in DevTools):
    - [ ] Projects page renders
    - [ ] Tasks kanban renders; status strips show correct colours (not hardcoded green/red/amber)
    - [ ] Task detail panel opens; header shows StatusBadge dot + task title + close button
    - [ ] Task detail: Diff / Logs / Attempts tabs (labeled, not icon-only)
    - [ ] Task detail footer: Merge (filled primary), Rebase (outline), Open in IDE (ghost)
    - [ ] Nodes tab in Navbar navigates to `/nodes`; node cards render
    - [ ] Board / Nodes / Processes tab row switches between views
    - [ ] Theme toggle flips dark ↔ light; token colours update correctly
    - [ ] Hard-reload after theme toggle: persists (confirms `updateAndSaveConfig` write path)
- SC18: Task-detail panel chrome aligned to ADR-0006: header renders `StatusBadge` dot + task title
  + close affordance; badges row (status outline+dot, node secondary, labels outline) below
  header; view switcher is labeled `Tabs` (Diff / Logs / Attempts), not an icon-only
  `ToggleGroup`; footer: Merge `variant="default" size="sm" className="flex-1"`,
  Rebase `size="sm"`, Open in IDE `variant="ghost" size="sm"`. Panel remains resizable —
  no 460px-fixed drawer, no overlay scrim.
- SC19: WCAG AA contrast: all text/background pairs introduced or changed meet 4.5:1 (normal text)
  and 3:1 (large text / UI components). Spot-check with browser DevTools accessibility panel
  on Projects, Tasks kanban, and Nodes pages for both dark and light themes.
- SC20: Theme persists across reload: after toggling to light, hard-reload of `/projects` still shows
  light theme (confirms the `updateAndSaveConfig` write, not just in-memory state).
- SC21: `/nodes` route exists in `frontend/src/App.tsx` inside `NormalLayout`, alongside `/processes`.
- SC22: Brand palette is live under `.dark` (scope decision C). In `frontend/src/styles/index.css`,
  the `.dark { }` block remaps the core theme tokens to the brand palette — at minimum
  `--_primary: var(--vks-cyan)` and `--_background: var(--vks-void)` — and the `--vks-*` primitives
  (`--vks-void`, `--vks-cyan`, `--vks-surface`, `--vks-surface-bright`, `--vks-emerald`,
  `--vks-amber`, `--vks-coral`) are defined under `:root` so they resolve in the applied cascade.
  Verify: `grep -A 40 '\.dark {' frontend/src/styles/index.css | grep -- '--_primary: var(--vks-cyan)'`
  → match. Runtime check (SC17): the VKSLogo "VK" (rendered with `text-primary`) appears cyan, not gray.

## Constraints

- No backend API changes in this workstream — UI only. No new endpoints, no Rust changes.
- No new `ThemeMode` enum variant; no modifications to `shared/types.ts`. Midnight Terminal maps
  to the existing `ThemeMode.DARK`. ThemeToggle uses `setTheme()` from the existing `useTheme()`
  hook, which calls `updateAndSaveConfig` internally — no new API endpoint.
- Must not break existing Playwright tests or introduce new console errors on the happy path.
- Remove `target/` and `node_modules/` between build iterations to conserve disk space.
- The design-source tree (`dev-docs/designs/2026-06-28-ui-overhaul/design-source/`) must not be
  modified.
- Task detail stays the resizable workbench (ADR-0006); do not introduce a Sheet/Drawer wrapper
  or 460px fixed geometry.
- All new visible strings (Board, Nodes, Processes tab labels; `+ Task` label; ThemeToggle
  aria-labels) may be English-only literals with a `// TODO(i18n): vk-swarm-node-ui-localize`
  comment. Formal i18n key wiring is deferred to the `vk-swarm-node-ui-localize` workstream.
- All token values added to `index.css` must be HSL channel triplets (`240 20% 5%`), not hex
  (`#0a0a0f`), for compatibility with Tailwind's `hsl(var(--...))` consumption pattern.
- `kanban/index.tsx` throughout this spec refers to the vendored file at
  `frontend/src/components/ui/shadcn-io/kanban/index.tsx`. This file is locally maintained (not
  auto-generated); edits to it are in scope and will not be overwritten by a shadcn-io update.

## Out of scope

- Backend swarm/hive sync changes (separate workstream: `vk-swarm-hive-redesign`).
- ANSI/BBS log viewer skin (type-code.html specimen — advanced feature, not in gap analysis).
- Drag-and-drop column reordering.
- i18n key wiring for new strings — English literals with `// TODO(i18n)` breadcrumbs are
  acceptable; formal wiring is deferred to `vk-swarm-node-ui-localize`.
- Mobile-first redesign — responsive behaviour in spec is desktop-primary with mobile adaptation,
  not a new mobile UX.
- `ThemeMode.SYSTEM` handling — the toggle cycles only `DARK ↔ LIGHT`; SYSTEM mode is untouched.
- Generating a new `ThemeMode` enum variant or modifying `shared/types.ts`.
- New Playwright e2e tests — Phase 4 verification is a manual browser smoke-test (D9); existing
  Playwright tests must not be broken.

## Approach

The 57 gap-analysis findings are sequenced so each layer consumes the one before it. Token work
lands first (everything downstream references it); component fidelity second; new surfaces last.
The 5 A-class bugs are folded in as early wins where they touch a layer already being edited.

**Phase 1 — Token foundation** (`frontend/src/styles/index.css`, `frontend/tailwind.config.js`).

> **Runtime-cascade correction (verified against `index.css`, scope decision C).** The `--vks-*`
> brand primitives and the `--_*`→brand remap currently live **only** in the `.vks-theme { }`
> block, which `ThemeProvider.tsx` **never applies to the DOM** (it only ever calls
> `root.classList.add('light'|'dark')`). The Midnight Terminal palette therefore renders
> **nowhere** today — the app runs the muted `--_*` values defined in `.dark { }`. To make the
> design system actually render (the workstream's intent), Phase 1 must **bring the brand palette
> into the live cascade**, not merely correct its dead values. New semantic/status/glow tokens
> that reference `var(--vks-*)` only resolve once those primitives exist in a selector that is
> actually applied (`:root` or `.dark`).

Steps:

- **Promote the `--vks-*` primitives to `:root` (live, both themes), with corrected values.**
  Define under `:root { }` so every theme can reference them (`.vks-theme` may keep its copies —
  it is exercised by `DesignSystem.test.tsx`; do not delete it, just stop relying on it):
  - `--vks-void`: `240 20% 5%` (was `240 33% 5%`)
  - `--vks-surface`: `240 18% 9%`
  - `--vks-surface-bright`: `240 16% 12%` (saturation `14%` → `16%`)
  - `--vks-cyan`: `190 100% 50%` (hue `193` → `190`)
  - `--vks-amber`: `43 100% 50%`, `--vks-emerald`: `152 100% 50%`,
    `--vks-coral`: `0 100% 71%`, `--vks-violet`: `270 91% 65%`
  - `--vks-text`: `240 6% 90%`, `--vks-text-muted`: `240 4% 46%`, `--vks-text-dim`: `240 3% 26%`
- **Merge the `.vks-theme` `--_*`→brand remap into the live `.dark { }` block** so dark mode = Midnight
  Terminal. Replace `.dark`'s muted `--_*` mappings with the brand references (mirroring the
  existing `.vks-theme` body): `--_background: var(--vks-void)`, `--_foreground: var(--vks-text)`,
  `--_muted: var(--vks-surface)`, `--_muted-foreground: var(--vks-text-muted)`,
  `--_primary: var(--vks-cyan)`, `--_primary-foreground: var(--vks-void)`,
  `--_secondary: var(--vks-surface-bright)`, `--_secondary-foreground: var(--vks-text)`,
  `--_accent: var(--vks-violet)`, `--_destructive: var(--vks-coral)`,
  `--_border: var(--vks-surface-bright)`, `--_input: var(--vks-surface)`, `--_ring: var(--vks-cyan)`,
  `--_success: var(--vks-emerald)`, `--_warning: var(--vks-amber)`, `--_info: var(--vks-cyan)`.
  Also set `--font-heading: 'Source Serif 4', Georgia, serif` under `.dark`. This is the (C)
  scope amendment — the whole dark chrome becomes cyan/void, so WCAG (SC19) is app-wide.
- **Create a `.light { }` block** (it does not exist today — light theme is the `:root` default).
  ThemeProvider applies `.light` via `classList.add('light')`, so a `.light { }` block WILL take
  effect. Put light-theme `--status-*` overrides here, plus a light primary tuned for AA contrast:
  `--_primary: 192 100% 35%` (#0091b5 teal). Light `--_background`/`--_foreground` stay at the
  `:root` defaults unless an override is needed for contrast.
- **Add semantic aliases** under `.dark { }`:
  - `--surface-card: var(--vks-surface)` (card/panel fill ≈ `#12121a`)
  - `--surface-raised: var(--vks-surface-bright)` (elevated surface ≈ `#1a1a24`)
  - `--border-strong: 240 10% 16%` (stronger border ≈ `#2a2a38`)
- **Add status tokens** — dark under `.dark { }`, light under `.light { }`:

  | Token | Dark HSL | Light HSL |
  |---|---|---|
  | `--status-todo` | `240 5% 63%` | `220 9% 46%` |
  | `--status-inprogress` | `217 91% 60%` | `221 83% 53%` |
  | `--status-inreview` | `43 100% 50%` | `38 100% 34%` |
  | `--status-done` | `152 100% 50%` | `153 83% 30%` |
  | `--status-cancelled` | `0 100% 71%` | `0 62% 52%` |

- **Add shadow/glow tokens** under `.dark { }` (values from `design-source/project/tokens/spacing.css`):
  - `--shadow-sm`, `--shadow-md`, `--shadow-lg`, `--glow-cyan`, `--glow-emerald`.
  - **Translation note:** the design-source glow tokens reference `hsl(var(--vks-cyan-hsl) ...)`
    and `hsl(var(--vks-emerald-hsl) ...)`. The product's primitive tokens are named `--vks-cyan`
    and `--vks-emerald` (no `-hsl` suffix). Rewrite the references to `hsl(var(--vks-cyan) / 0.4)`
    etc. (now resolvable because the primitives are promoted to `:root`).
- **Add `--strip-width: 4px`** (global scope, not inside a theme selector)
- **Add `font-wordmark`** to `tailwind.config.js` `theme.fontFamily` (note: a `'chivo-mono'` key
  already exists — add `wordmark` as the spec'd alias VKSLogo will consume via `font-wordmark`):
  ```js
  wordmark: ["'Chivo Mono'", 'monospace'],
  ```
  `code`/`mono` already map to JetBrains Mono (`var(--font-code)`); `serif`/`heading` already
  map to Source Serif 4 — verify, do not re-add.
- **Add ANSI texture utility classes** to `index.css` (used by empty-states):
  ```css
  .vks-ansi-dither { /* radial-gradient dot field — copy from design-source specimens */ }
  .vks-ansi-dither-dense { /* denser dot variant */ }
  .vks-scanlines { /* ::after CRT overlay: scanlines + vignette */ }
  ```
  Copy the CSS bodies verbatim from `design-source/project/guidelines/` or the specimen HTML
  files in `design-source/`. Do not invent values.
- **Add `@keyframes vks-pulse`**:
  ```css
  @keyframes vks-pulse {
    0%, 100% { opacity: 1; box-shadow: 0 0 0 0 hsl(152 100% 50% / 0.6); }
    50%       { opacity: 0.7; box-shadow: 0 0 0 4px hsl(152 100% 50% / 0); }
  }
  ```
  Use `hsl(152 100% 50%)` directly (or `hsl(var(--status-done))`). Do **not** reference
  `--vks-emerald-hsl` — that token is not defined anywhere. Duration: `2s`, easing:
  `ease-in-out`, iteration-count: `infinite`.

**Phase 2 — Component fidelity** (token-consuming). Switch hardcoded Tailwind colours to
the new tokens; correct typography/geometry per `design-spec.md`. A-class bugs ship here.

> **Canonical file paths** (verified against current repo structure):
>
> | Short name | Full verified path |
> |---|---|
> | `TaskCard.tsx` | `frontend/src/components/tasks/TaskCard.tsx` |
> | `AllProjectsTaskCard.tsx` | `frontend/src/components/tasks/AllProjectsTaskCard.tsx` |
> | `TaskCardHeader.tsx` | `frontend/src/components/tasks/TaskCardHeader.tsx` |
> | `TaskCountPills.tsx` | `frontend/src/components/projects/TaskCountPills.tsx` (**not** `tasks/`) |
> | `kanban/index.tsx` | `frontend/src/components/ui/shadcn-io/kanban/index.tsx` |
> | `DaysInColumnBadge.tsx` | `frontend/src/components/tasks/DaysInColumnBadge.tsx` |
> | `LabelBadge.tsx` | `frontend/src/components/tasks/LabelBadge.tsx` |
> | `VKSLogo.tsx` | `frontend/src/components/VKSLogo.tsx` (**not** `ui/`) |
> | `Logo.tsx` (current, to be replaced in Navbar) | `frontend/src/components/Logo.tsx` |
> | `AttemptHeaderActions.tsx` | `frontend/src/components/panels/AttemptHeaderActions.tsx` |
> | `GitOperations.tsx` | `frontend/src/components/tasks/Toolbar/GitOperations.tsx` |
> | `SearchBar.tsx` | `frontend/src/components/SearchBar.tsx` |
> | `LabelBadge.tsx` | `frontend/src/components/labels/LabelBadge.tsx` (**not** `tasks/`) |
> | `daysInColumn.ts` | `frontend/src/utils/daysInColumn.ts` |
> | `NodeProjectsSection.tsx` | `frontend/src/components/swarm/NodeProjectsSection.tsx` |
> | existing nodes hook | `frontend/src/hooks/useAvailableNodes.ts` (returns `ListProjectNodesResponse`) |

Component changes:

- **`TaskCard.tsx` + `AllProjectsTaskCard.tsx`**: Replace the `statusStripColors` map
  (`bg-green-500`, `bg-red-500`, `bg-amber-500`, `bg-blue-500`) with `bg-[hsl(var(--status-done))]`
  etc. (or a map to `var(--status-*)` string values). Title → `font-medium text-base` (currently
  `font-light text-sm` in `TaskCardHeader.tsx`). Description → `text-sm` (was `text-xs`). Remove
  JS pre-truncation in `truncateDescription` utility — rely on CSS `line-clamp`/`truncate`.
  Node tag: add `font-mono`. `AttemptIndicator` merged state: `text-success` (was `text-green-500`).
- **`TaskCountPills.tsx`** (`frontend/src/components/projects/TaskCountPills.tsx`):
  **(A-class bug)** Swap colours: inprogress → blue HSL `217 91% 60%`, inreview → amber HSL
  `43 100% 50%`. Verify exact lines before editing (approximately lines 40 and 49 — use grep).
- **`DaysInColumnBadge.tsx` / `daysInColumn.ts`**: Flat `secondary` badge variant; return
  literal `{n}d` format with no cap (remove `7d+` ceiling).
- **`LabelBadge.tsx`**: Add outline variant for task-card context.
- **`frontend/src/components/ui/shadcn-io/kanban/index.tsx`**:
  - Selected ring: `ring-2 ring-primary` (drop `ring-inset ring-secondary-foreground`, ~line 118)
  - Column add button **(A-class bug)**: `h-0` → `h-6 w-6` ghost icon button (~lines 243–250)
  - Status dot: `h-[9px] w-[9px]` (was `h-2.5`, ~line 220)
  - Count badge background: `bg-[hsl(var(--surface-card))]` (was `bg-muted`, ~lines 226–232)
- **`VKSLogo.tsx`**: Apply `font-wordmark` Tailwind class to the wordmark text (`className="font-wordmark"`).
  Remove any `font-code`/`font-mono` currently applied to it.
- **`ThemeToggle` (new component)**: Ghost icon button (sun ↔ moon, 28–32px). Calls
  `setTheme(ThemeMode.DARK)` or `setTheme(ThemeMode.LIGHT)` from the existing `useTheme()` hook.
  Cycles `DARK ↔ LIGHT` only — does not touch `ThemeMode.SYSTEM`. No new API call; `setTheme`
  internally calls `updateAndSaveConfig`. Add `aria-label` with a
  `// TODO(i18n): vk-swarm-node-ui-localize` comment.

**Phase 3 — Board & surfaces.** Geometry + new surfaces.

- **`frontend/src/components/ui/shadcn-io/kanban/index.tsx`**:
  - Column grid: `auto-cols-[minmax(264px,1fr)]` (was `auto-cols-[minmax(200px,400px)]`)
  - Empty-state: add a `div` with classes `vks-ansi-dither vks-scanlines rounded-md border min-h-[80px] flex items-center justify-center` containing text `░▒ no tasks ▒░` in `font-mono text-xs text-muted-foreground`.
- **`frontend/src/components/layout/Navbar.tsx`**:
  - Swap `<Logo>` SVG → `<VKSLogo>` component.
  - `+ Task` → `<Button variant="default" size="sm">+ Task</Button>`; add
    `{/* TODO(i18n): vk-swarm-node-ui-localize */}` comment on the label string.
  - Add second `<nav>` row below the main bar. Tab links (each with
    `// TODO(i18n): vk-swarm-node-ui-localize` on the label):
    - **Board** → `/projects/${lastVisitedProjectId}/tasks`; if no `lastVisitedProjectId` in
      `localStorage`, link falls back to `/projects`. Tab is visually active when
      `pathname.startsWith('/projects/')`. Track `lastVisitedProjectId` in `localStorage` on
      every successful project navigation.
    - **Nodes** → `/nodes` (new global route).
    - **Processes** → `/processes` (existing global route).
    - Active tab style: `border-b-2 border-primary` with `mb-[-1px]` so the 2px underline
      bleeds into the main nav bottom border.
  - Add `<ThemeToggle>` (from Phase 2) in the right action cluster.
  - `ProjectSwitcher.tsx`: add `<FolderOpen>` icon inside the trigger button.
  - `SearchBar.tsx`: width → `w-[260px]`.
- **Nodes view (new)**:
  - `frontend/src/pages/Nodes.tsx` — new page component. Layout: responsive grid
    `grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3 max-w-[1000px]`. Heading
    `<h2 className="font-display text-2xl font-semibold">`. Fetch data via an existing nodes
    hook — search `frontend/src/hooks/` for `useNodes`, `useSwarmNodes`, or similar before
    creating a new one. Show a `NodeCard` per node.
  - `frontend/src/components/swarm/NodeCard.tsx` — new presentational component. Props:
    `{ node: SwarmNode; className?: string }`. Row layout: OS-glyph container (36×36px, raised
    surface background, `rounded-md`), `font-mono` node name, online pulse dot (8×8px,
    `bg-[hsl(var(--status-done))]`, `animate-[vks-pulse_2s_ease-in-out_infinite]`) or offline dim
    dot (`bg-[hsl(var(--vks-text-dim))]`, no animation), right slot for agent-count badge or offline badge.
  - `frontend/src/App.tsx`: add `<Route path="/nodes" element={<Nodes />} />` inside
    `NormalLayout`, alongside the existing `<Route path="/processes" element={<Processes />} />`.
- **Task-detail panel chrome** (`AttemptHeaderActions.tsx`, `GitOperations.tsx`):
  - Replace icon-only `ToggleGroup` with labeled `<Tabs>` component. Tab mapping:

    | Tab label | `LayoutMode` value | Action |
    |---|---|---|
    | Diff | `'diffs'` | Sets `mode('diffs')` |
    | Logs | `'terminal'` | Sets `mode('terminal')` |
    | Attempts | (reset to null) | Sets `mode(null)`; attempt pane shows attempt-history list |

    "Attempts" resets `mode` to `null` — no new `LayoutMode` value needed (D6).
  - Header: add `<StatusBadge>` dot + task title (`text-lg font-semibold`) + close `X` button.
  - Below header: badges row — status badge (outline + dot), node badge (secondary), label
    badges (outline).
  - Footer: Merge `variant="default" size="sm" className="flex-1"` ·
    Rebase `size="sm"` · Open in IDE `variant="ghost" size="sm"`.

**Phase 4 — Verification.**

Run the static CI gates (SC1–SC4), then the greppable assertions (see Test strategy), then the
manual smoke-test checklist (SC17).

Start the dev server:
```bash
pnpm run dev   # from worktree root; backend auto-assigns port recorded in .env; frontend → :3000
```
Open `http://localhost:3000` and work through every SC17 checklist item. Use browser DevTools to
confirm zero console errors and spot-check WCAG AA contrast (SC19). After theme toggle, hard-reload
and confirm persistence (SC20). When done:
```bash
rm -rf target node_modules frontend/node_modules   # conserve disk between iterations
```

## Design / architecture

**Token system.** `index.css` already layers `--vks-*` brand primitives → semantic aliases
(`--background`, `--primary`, …) → Tailwind config consumption. New tokens extend this pattern
with one **critical correction from the panel review**: all new definitions must go under the
**`.dark { }` selector** (the class `ThemeProvider.tsx` actually applies). The `.vks-theme { }`
selector is **dead code** — `ThemeProvider` never sets that class — and must not receive new
tokens. Light overrides go under `.light { }`, not `[data-theme="light"]`.

Components reference tokens via Tailwind arbitrary values (`bg-[hsl(var(--status-done))]`, since the
token is a bare HSL triplet) or a
`statusStripColors` map keyed by status, so the five status colours resolve through one source
of truth per theme.

**Theme model.** "Midnight Terminal" dark theme = `ThemeMode.DARK` = the existing `.dark` CSS
class. `ThemeProvider.tsx` already applies this via `root.classList.add('dark')`. No new enum
variant is added (D4, D5). `ThemeToggle` calls `setTheme(ThemeMode.DARK | ThemeMode.LIGHT)`,
which triggers `updateAndSaveConfig` — the same path as General Settings. This persists to the
backend `Config.theme` field and survives page reload (SC20). Console tokens (`--console-*`)
are intentionally never overridden in light mode.

**Navigation / tab row.** Tab row is top-level view navigation, additive:
- **Board** → `/projects/:lastProjectId/tasks` (last-visited project in `localStorage`;
  falls back to `/projects`). Active when `pathname.startsWith('/projects/')`.
- **Processes** → `/processes` (existing global route, `App.tsx`). The per-attempt
  processes aux pane (`LayoutMode='processes'`) is a separate, untouched surface.
- **Nodes** → `/nodes` (new global route). Only genuinely new navigation target.

**Tabs mapping — task detail (ADR-0006, D6):**

| Tab label | `LayoutMode` value | Behaviour |
|---|---|---|
| Diff | `'diffs'` | Existing diff viewer aux pane |
| Logs | `'terminal'` | Existing terminal/log aux pane |
| Attempts | null (reset) | `mode` reset to `null`; attempt-history list renders in attempt pane |

"Attempts" does not add a new `LayoutMode` variant — it navigates to the existing `mode=null`
state where the attempt-history list already renders. D4 (no generated-type changes) stays intact.

**File path register** (canonical, verified against repo):

| Short name used in spec | Full verified path | Notes |
|---|---|---|
| `kanban/index.tsx` | `frontend/src/components/ui/shadcn-io/kanban/index.tsx` | Vendored, locally maintained — edits OK |
| `TaskCountPills.tsx` | `frontend/src/components/projects/TaskCountPills.tsx` | **Not** in `tasks/` |
| `AllProjectsTaskCard.tsx` | `frontend/src/components/tasks/AllProjectsTaskCard.tsx` | Has same status-strip bug as TaskCard |
| `AttemptHeaderActions.tsx` | `frontend/src/components/panels/AttemptHeaderActions.tsx` | |
| `GitOperations.tsx` | `frontend/src/components/panels/GitOperations.tsx` | Verify sub-path before editing |
| `TasksLayout.tsx` | `frontend/src/components/layout/TasksLayout.tsx` | |
| `ThemeProvider.tsx` | `frontend/src/components/ThemeProvider.tsx` | Only ever sets `.light`/`.dark` — never `.vks-theme` |

> **Gap-analysis path note:** the gap-analysis cites `components/tasks/TaskCountPills.tsx:45,52`
> and `kanban/index.tsx` without the `ui/shadcn-io/` prefix. Both paths are stale. Use the
> verified paths in the table above.

**NodeCard.** New presentational component; data-shape-compatible with what `NodeProjectsSection.tsx`
already renders. Check `frontend/src/hooks/` for an existing node-listing hook before creating
one. No backend or type changes — UI only.

**No Rust / API changes.** Every change is in `frontend/src` plus `index.css`/`tailwind.config.js`.
`npm run generate-types` is not triggered. Backend CI gates must pass and are unaffected.

## Decisions

- **D1 — Keep the resizable task-detail workbench; do not build the 460px drawer.**
  *Irreversible? No (reversible — nothing deleted).* Recorded as ADR-0006.
  → `dev-docs/adr/0006-task-detail-retain-workbench-over-drawer.md`
- **D2 — Board/Nodes/Processes tab row is additive top-level navigation, not a relocation.**
  Processes tab → existing `/processes`; Nodes is the only new view. Reversible; no ADR.
- **D3 — Status colours via five new `--status-*` tokens (dark + light), not hardcoded utilities.**
  Mirrors `--success`/`--warning`/`--danger`; one source of truth per status across themes.
  Reversible; no ADR.
- **D4 — UI-only; no backend, API, DB, or generated-type changes.** Reversible; no ADR.
- **D5 — "Midnight Terminal" maps to the existing `ThemeMode.DARK` path; no new enum variant.**
  `ThemeToggle` uses `setTheme(ThemeMode.DARK|LIGHT)` via existing `useTheme()` +
  `updateAndSaveConfig`. Collapsing onto `.dark` avoids a Rust + ts-rs type change that would
  violate D4. Reversible; no ADR.
- **D6 — "Attempts" tab in task detail resets `mode` to `null`; no new `LayoutMode` value.**
  Attempt-history already renders at `mode=null`. No union extension, no type-generation
  implications. Reversible; no ADR.
- **D7 — New UI strings are English-only literals with `// TODO(i18n): vk-swarm-node-ui-localize`
  breadcrumbs.** Formal i18n wiring deferred to the named follow-up workstream. Keeps this
  workstream UI-only and avoids i18n churn for four strings. Reversible; no ADR.
- **D8 — Nodes global route is `/nodes`**, consistent with existing `/processes`. Board tab tracks
  `lastVisitedProjectId` in `localStorage` with `/projects` fallback. Reversible; no ADR.
- **D9 — Phase 4 verification is a manual browser smoke-test, not a new Playwright suite.**
  No Playwright config, `e2e/` directory, or base-URL spec exists for this workstream. Existing
  Playwright tests must not break (constraint). New Playwright tests are out of scope. SC17
  manual checklist is the acceptance gate. Reversible; no ADR.
- **D10 — Scope (C): make the Midnight Terminal brand palette live under `.dark`.** Decompose
  discovered the `--vks-*` palette + `--_*`→brand remap live only in the `.vks-theme { }` block,
  which `ThemeProvider` never applies — so the design system renders nowhere today (the app is
  muted gray). The user chose to deliver the full design (not the SC-minimal token-only pass).
  Phase 1 therefore promotes the `--vks-*` primitives to `:root` and merges the remap into the
  live `.dark` block; SC22 gates it. Blast radius is app-wide (every dark surface re-colours),
  so SC19 (WCAG AA) is evaluated app-wide. `.vks-theme` is retained (not deleted) because
  `DesignSystem.test.tsx` references the class. *Irreversible? No — reversible via git; nothing is
  deleted, the remap mirrors the already-present `.vks-theme` body.* No ADR (reversible, and the
  decision is recorded here + in the decisions-ledger). This amendment re-froze the spec via a
  second `/wai:precheck` run.

## Test strategy

**Static gates (CI parity):**
```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
```
All four must be green (SC1–SC4).

**Greppable assertions** (run from repo root, each must match expected result):
```bash
# SC5a — old hardcoded status colours gone
grep -r 'bg-green-500\|bg-red-500\|bg-amber-500\|bg-blue-500' \
  frontend/src/components/tasks/ \
  frontend/src/components/projects/TaskCountPills.tsx
# expected: zero matches

# SC5b — token replacement confirmed in key files
grep -r 'var(--status-' \
  frontend/src/components/tasks/TaskCard.tsx \
  frontend/src/components/tasks/AllProjectsTaskCard.tsx
# expected: ≥1 match per file

# SC6 — status tokens defined under .dark (not .vks-theme)
grep -A 100 '\.dark {' frontend/src/styles/index.css | grep 'status-todo'
# expected: match

# SC7 — semantic tokens defined (dark + light overrides → 6 declarations)
grep -E '\-\-(border-strong|surface-card|surface-raised)' frontend/src/styles/index.css
# expected: ≥3 matches (6 with the .light overrides)
# SC7 — and consumed by a component (border-strong via TaskCard hover; surface-card via kanban/NodeCard)
grep -rE 'hsl\(var\(--(border-strong|surface-card|surface-raised)\)\)' frontend/src/components
# expected: ≥1 match per token

# SC9 — kanban column min-width
grep 'minmax(264px' frontend/src/components/ui/shadcn-io/kanban/index.tsx
# expected: match

# SC12 — VKSLogo in Navbar
grep '<VKSLogo' frontend/src/components/layout/Navbar.tsx
# expected: match

# SC14 / SC21 — NodeCard and /nodes route
ls frontend/src/components/swarm/NodeCard.tsx
grep 'path="/nodes"' frontend/src/App.tsx
# expected: both present

# SC15 — vks-pulse keyframe
grep 'vks-pulse' frontend/src/styles/index.css
# expected: match (keyframe definition)
```

**Component behaviour:** TaskCountPills swap (SC8) verified by reading corrected source; extend
any existing tests covering status colour mapping. No new Rust tests required.

**Manual browser smoke-test** (SC17 checklist — see Success criteria §17):
Start: `pnpm run dev` from worktree root. Open `http://localhost:3000` (or port from `.env`).
Also spot-check WCAG AA (SC19) via browser DevTools accessibility panel on Projects, Tasks,
Nodes pages in both themes. Verify SC20 (theme persists on hard-reload). Remove artifacts
between iterations:
```bash
rm -rf target node_modules frontend/node_modules
```

**Visual fidelity:** spot-check against `design-spec.md` token tables and component anatomies.
The greppable assertions above are the acceptance bar; subjective "looks right" is insufficient.
