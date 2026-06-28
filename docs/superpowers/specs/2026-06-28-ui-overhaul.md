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
6. The task detail surface presents as a right-anchored drawer or closely approximates the spec
   layout (drawer vs. existing resizable panel is an explicit decision to be made in `/wai:spec`).
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

## Constraints

- No backend API changes in this workstream — UI only.
- Must not break existing Playwright tests or introduce new console errors on the happy path.
- `target/` and `node_modules/` should be removed between build iterations to control disk use.
- Remove `target` and `node_modules` directories between iterations to conserve disk space.
- The design-source tree (`dev-docs/designs/2026-06-28-ui-overhaul/design-source/`) must not be modified.
- Drawer-vs-panel architecture decision (SC6) must be made explicitly before implementation.

## Out of scope

- Backend swarm/hive sync changes (separate workstream: `vk-swarm-hive-redesign`).
- ANSI/BBS log viewer skin (type-code.html specimen — advanced feature, not in gap analysis).
- Drag-and-drop column reordering.
- i18n changes (separate workstream: `vk-swarm-node-ui-localize`).
- Mobile-first redesign — responsive behaviour in spec is desktop-primary with mobile adaptation, not a new mobile UX.

## Approach

*(To be filled in by `/wai:spec ui-overhaul` — see design reference above.)*

## Design

*(To be filled in by `/wai:spec ui-overhaul`)*
