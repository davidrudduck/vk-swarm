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
5. A `NodeCard` component exists and is used in a Nodes view rendered on the Nodes tab.
6. The task detail surface keeps its resizable multi-pane workbench (diff / terminal / files /
   processes), with the in-panel chrome aligned to the drawer spec — see ADR-0006
   (`dev-docs/adr/0006-task-detail-retain-workbench-over-drawer.md`).
7. Light and dark themes are fully token-driven; the dark theme ("Midnight Terminal") is the brand
   default; light theme is an opt-in alternate.
8. No console/linting errors introduced. All CI checks pass.

## Users / who is affected

- All VK-Swarm users — visual appearance of every primary surface changes.
- Developers — component API contracts may change for `TaskCard`, `Badge`, `Button`, `NodeCard`.

## Success criteria

Testable definitions of "done":

1. `cargo clippy --all --all-targets --all-features -- -D warnings` passes.
2. `cargo test --workspace` passes.
3. `cd frontend && npm run lint` passes (zero ESLint errors).
4. `cd frontend && npx tsc --noEmit` passes.
5. `css` audit: `grep -r 'bg-green-500\|bg-red-500\|bg-amber-500\|bg-blue-500' frontend/src/components/tasks/` returns zero matches.
6. `--status-todo`, `--status-inprogress`, `--status-inreview`, `--status-done`, `--status-cancelled` are defined in `frontend/src/styles/index.css` and used by TaskCard status strips.
7. `--border-strong`, `--surface-card`, `--surface-raised` are defined and consumed.
8. `TaskCountPills` inprogress→blue, inreview→amber (was swapped).
9. Kanban column min-width: `minmax(264px, 1fr)` (was `minmax(200px,400px)`).
10. Empty column renders ansi-dither + scanlines empty state (not blank).
11. Navbar second row has Board / Nodes / Processes tabs with active 2px cyan underline.
12. `<VKSLogo>` rendered in Navbar (not SVG `<Logo>`).
13. `+ Task` button in Navbar: `variant="default" size="sm"` with text label.
14. `NodeCard` component exists in `frontend/src/components/swarm/NodeCard.tsx` and renders in a Nodes view.
15. `vks-pulse` keyframe is defined in index.css.
16. A theme toggle (sun/moon) is present in the Navbar.
17. Manual browser smoke-test: Projects page, Tasks kanban, task detail panel all render without console errors; Nodes tab shows node cards.
18. Task-detail panel chrome aligned to ADR-0006: header renders `StatusBadge` dot + task title + close affordance; a badges row (status outline+dot, node secondary, labels outline) sits below the header; the view switcher is labeled `Tabs` (Diff / Logs / Attempts), not an icon-only `ToggleGroup`; footer shows Merge (`variant="default" size="sm"` + `flex-1`), Rebase (`size="sm"`), Open in IDE (`variant="ghost" size="sm"`). The panel remains resizable — no 460px-fixed drawer, no overlay scrim.

## Constraints

- No backend API changes in this workstream — UI only.
- Must not break existing Playwright tests or introduce new console errors on the happy path.
- `target/` and `node_modules/` should be removed between build iterations to control disk use.
- Remove `target` and `node_modules` directories between iterations to conserve disk space.
- The design-source tree (`dev-docs/designs/2026-06-28-ui-overhaul/design-source/`) must not be modified.
- Task detail stays the resizable workbench (ADR-0006); do not introduce a Sheet/Drawer wrapper or 460px fixed geometry.

## Out of scope

- Backend swarm/hive sync changes (separate workstream: `vk-swarm-hive-redesign`).
- ANSI/BBS log viewer skin (type-code.html specimen — advanced feature, not in gap analysis).
- Drag-and-drop column reordering.
- i18n changes (separate workstream: `vk-swarm-node-ui-localize`).
- Mobile-first redesign — responsive behaviour in spec is desktop-primary with mobile adaptation, not a new mobile UX.

## Approach

The 57 gap-analysis findings are sequenced so each layer consumes the one before it. Token work
lands first (everything downstream references it); component fidelity second; new surfaces last.
The 5 A-class bugs are folded in as early wins where they touch a layer already being edited.

**Phase 1 — Token foundation** (`frontend/src/styles/index.css`, `frontend/tailwind.config.js`).
Add the missing semantic + status tokens and fix the wrong HSL values. Nothing renders
differently yet except where a token value changed, but every later phase depends on these names
existing. Includes the `vks-pulse` keyframe and the `--strip-width` custom property.

- Fix values: `--vks-void` → `240 20% 5%` (#0a0a0f); `--vks-surface-bright` saturation → `16%`;
  `--vks-cyan` hue → `190` (#00d4ff). Light `:root` `--_primary` → `192 100% 35%` (#0091b5).
- Add semantic aliases: `--surface-card`, `--surface-raised`, `--border-strong`.
- Add status tokens (dark + light): `--status-todo|inprogress|inreview|done|cancelled`.
- Add `@keyframes vks-pulse` (2s) and `--strip-width: 4px`.

**Phase 2 — Component fidelity** (token-consuming). Each component below switches hardcoded
Tailwind colours to the new tokens and corrects typography/geometry per `design-spec.md`. The
A-class bugs in this layer (TaskCountPills colour swap, TaskCard double-truncation) ship here.

- `TaskCard.tsx` / `TaskCardHeader.tsx`: status strip → `var(--status-*)`; title
  `font-medium text-base`; description `text-sm`, remove JS pre-truncation; node tag `font-code`;
  AttemptIndicator `text-success`.
- `TaskCountPills.tsx`: **(A)** swap inprogress→blue, inreview→amber.
- `DaysInColumnBadge.tsx` / `daysInColumn.ts`: flat `secondary` variant, literal `{n}d` (no cap).
- `LabelBadge.tsx`: outline variant for task-card context.
- `kanban/index.tsx`: selected ring → `ring-2 ring-primary` (drop `ring-inset`); column add
  button **(A)** `h-0` → `h-6 w-6` ghost icon; status dot `h-[9px] w-[9px]`; count badge
  `--surface-card`.
- `VKSLogo.tsx`: Chivo Mono wordmark (`--font-wordmark`).

**Phase 3 — Board & surfaces.** Geometry + new surfaces that compose the styled components.

- `kanban/index.tsx`: column grid `minmax(264px, 1fr)`; empty-state `.vks-ansi-dither.vks-scanlines`
  block with `░▒ no tasks ▒░`.
- `Navbar.tsx`: swap `<Logo>` → `<VKSLogo>`; `+ Task` → `variant="default" size="sm"` + label;
  add the Board / Nodes / Processes tab row (active = 2px cyan underline); add `ThemeToggle`
  (sun/moon ghost icon). `ProjectSwitcher`/`SearchBar` minor geometry fixes.
- **Nodes view (new):** `NodeCard.tsx` in `frontend/src/components/swarm/` per the spec anatomy
  (OS glyph, `font-code` name, online `vks-pulse` dot / offline dim dot), rendered in a Nodes
  view reachable from the new tab.
- Task-detail panel internals per ADR-0006 (header, badges row, labeled tabs, footer styling).

**Phase 4 — Verification.** `cargo clippy`/`cargo test`/`npm run lint`/`tsc --noEmit` green;
Playwright smoke test of Projects / Tasks kanban / task-detail / Nodes tab with zero console
errors. Per the operating constraint, remove `target/` and `node_modules/` between build
iterations to conserve disk space.

## Design / architecture

**Token system.** `index.css` already layers `--vks-*` brand primitives → semantic aliases
(`--background`, `--primary`, …) → Tailwind config consumption. New tokens follow the same three
tiers: status colours are defined on `.vks-theme`/`:root` (dark) and overridden under the light
theme selector, exactly mirroring the existing `--success`/`--warning`/`--danger` pattern. No new
mechanism is introduced — only new names. Components reference tokens via Tailwind arbitrary
values (`bg-[var(--status-done)]`) or a small `statusStripColors` map keyed by status, so the
five status colours resolve through one source of truth in both themes.

**Theme model.** Dark ("Midnight Terminal") is the default `:root`; light is an opt-in alternate
selected by the existing theme mechanism. The new `ThemeToggle` drives that mechanism. Console
tokens (`--console-*`) are intentionally never overridden in light mode (log/diff viewers stay
dark) — preserved as-is.

**Navigation / tab row.** The design's Board / Nodes / Processes tab row is **top-level view
navigation**, additive — it does not relocate the existing per-attempt `mode: 'processes'` aux
pane. Mapping:
- **Board** → existing tasks/kanban view (`ProjectTasks`).
- **Processes** → existing top-level `/processes` route (`App.tsx:154`, `<Processes />`). The
  finer-grained per-attempt processes aux pane (`LayoutMode = 'processes'`, `ProjectTasks.tsx:983`)
  is a *different* surface and is left untouched.
- **Nodes** → **new** top-level view backed by the new `NodeCard`. This is the one genuinely new
  navigation target.

**Task detail (ADR-0006).** The resizable `kanban │ attempt │ aux` workbench in
`TasksLayout.tsx` is retained. Only the right-work-area chrome is restyled to the drawer spec:
header (`StatusBadge` + title + close), badges row, labeled Diff/Logs/Attempts `Tabs` replacing
the icon `ToggleGroup` in `AttemptHeaderActions.tsx`, and the `GitOperations.tsx` footer button
variants. No Sheet wrapper, no overlay, no fixed 460px width, no slide animation — those are
explicitly dropped (see ADR-0006 "Dropped" list).

**NodeCard.** New presentational component in `frontend/src/components/swarm/NodeCard.tsx`
(row layout, OS glyph in 36×36 raised container, `font-code` name, online/offline pulse dot). It
is data-shape-compatible with what `NodeProjectsSection.tsx` already renders; no backend or type
changes — UI only, per the workstream constraint.

**No Rust / API changes.** Every change is in `frontend/src` plus `index.css`/`tailwind.config.js`.
`npm run generate-types` is not triggered (no `#[derive(TS)]` structs touched). Backend CI gates
(`cargo clippy`, `cargo test`) must still pass but should be unaffected.

## Decisions

- **D1 — Keep the resizable task-detail workbench; do not build the 460px drawer.**
  *Irreversible? No (reversible — nothing deleted).* Recorded as an ADR for convention parity and
  to settle the divergence-from-design-source question once.
  → **ADR-0006** (`dev-docs/adr/0006-task-detail-retain-workbench-over-drawer.md`).
- **D2 — Board/Nodes/Processes tab row is additive top-level navigation, not a relocation.**
  Processes tab points at the existing `/processes` route; the per-attempt processes aux pane is
  untouched; Nodes is the only new view. Reversible; no ADR.
- **D3 — Status colours flow through five new `--status-*` tokens (dark + light), not hardcoded
  Tailwind utilities.** Mirrors the existing `--success`/`--warning`/`--danger` pattern; one
  source of truth per status across themes. Reversible; no ADR.
- **D4 — UI-only; no backend, API, DB, or generated-type changes.** Keeps the workstream scoped
  to the frontend and avoids `generate-types` churn. Reversible; no ADR.

## Test strategy

- **Static gates (CI parity):** `cargo clippy --all --all-targets --all-features -- -D warnings`,
  `cargo test --workspace`, `cd frontend && npm run lint`, `npx tsc --noEmit` — all green
  (success criteria 1–4).
- **Greppable assertions** (cheap, decompose-verifiable):
  - `grep -r 'bg-green-500\|bg-red-500\|bg-amber-500\|bg-blue-500' frontend/src/components/tasks/`
    → zero matches (SC5).
  - `--status-*`, `--border-strong`, `--surface-card`, `--surface-raised`, `vks-pulse` present in
    `index.css` (SC6, SC7, SC15).
  - `minmax(264px, 1fr)` in `kanban/index.tsx` (SC9); `<VKSLogo` referenced in `Navbar.tsx`
    (SC12); `NodeCard.tsx` exists under `frontend/src/components/swarm/` (SC14).
- **Component/behaviour:** TaskCountPills colour mapping (SC8) and the other A-bugs verified by
  reading the corrected source and, where a test exists, extending it. No new Rust tests required
  (UI-only).
- **Playwright smoke test (happy path, zero console errors):** Projects page, Tasks kanban,
  task-detail panel, and the new Nodes tab all render; tab row switches Board/Nodes/Processes;
  theme toggle flips dark↔light (SC10, SC11, SC16, SC17, SC18). Run against a fresh dev DB, not
  production. Between iterations, remove `target/` and `node_modules/` to conserve disk.
- **Visual fidelity:** spot-checked against `design-spec.md` token tables and component anatomies;
  the spec's concrete class/value checks are the acceptance bar, not subjective "looks right".
