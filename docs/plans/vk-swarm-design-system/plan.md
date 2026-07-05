# Plan: vk-swarm-design-system

Spec: `docs/superpowers/specs/2026-07-04-vk-swarm-design-system.md` (frozen, sha `cd78aed7a83e638941a7c81373459700461347eb`).
Workstream: `dev-docs/workstreams/vk-swarm-design-system/README.md`.
Design source: `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/` (preserved verbatim).

## Approach

The hive `remote-frontend/` ships only an auth shell (tasks 100-106 of `vk-swarm-hive-ui`: ProfileProvider, oauthApi, useAuth, slim NormalLayout/Navbar/BottomNav, AppRouter, root providers). The design bundle (`dev-docs/designs/.../design-source/`) specifies a complete Midnight Terminal design system: a CSS-token layer (color/typography/spacing/radius/motion/texture), a set of class-based component primitives (`.vks-*`), 9 core + 3 board + 2 settings React components, and an app UI kit (BoardView/Chrome/TaskDrawer/Panels) that wires those components to live data. The gap analysis (`dev-docs/designs/.../gap-analysis.md`) classified 31 findings: 1 A, 7 B, 23 C — i.e. almost everything is un-built in `remote-frontend/`, while the node `frontend/` already carries a partial token subset (11 `--vks-*` colors, 2 texture utilities, Tailwind `vks.*` namespace) that must NOT be touched (SC9).

This plan ports the design system into the hive `remote-frontend/` only. The approach is strictly layered so each phase is independently shippable: tokens → components → app UI kit + shell integration. The token CSS files are copied byte-identical from the preserved `design-source/tokens/` (the `.prettierignore` pattern established in task 201 applies — generated/preserved files must not be reformatted). React components are TypeScript ports of the JSX siblings in `design-source/components/{core,board,settings}/` and `design-source/ui_kits/vk-swarm-app/` — the JSX files are the authoritative anatomy; the port translates `className` string composition to `cn()` (already in `remote-frontend/src/lib/utils.ts` from task 104), adds explicit prop types from the `.d.ts` siblings, and keeps the `vks-*` class names verbatim. The app UI kit (phase 3) wires the components to the live REST + Electric data planes already available in `remote-frontend/` (from `vk-swarm-hive-ui` phase 1): `ProfileProvider`/`useAuth` for session, `nodesApi`-equivalent clients to be added against the hive `/v1/*` routes, and the Electric collections (existing 3 + new 3 from the abandoned tasks 300-308 — but only the 3 that are actually synced to Electric per `frontend/src/lib/electric/config.ts`'s `ELECTRIC_SHAPE_TABLES`: nodes, projects, node_projects, node_task_assignments, node_task_output_logs, node_task_progress_events). Phase 3 also integrates the new UI kit into the existing `AppRouter.tsx` route tree, replacing the slim task-104 `NormalLayout`/`Navbar`/`BottomNav` with the design-system `Chrome`, and swapping the `/nodes` and `/tasks` placeholders for `NodesView` and `BoardView`/`TaskDrawer` respectively.

SC9 (node frontend unmodified) is enforced mechanically: every task's Done-when gate chains `cd frontend && npx tsc --noEmit` (the node frontend typecheck) so any accidental edit is caught. No task touches `frontend/`.

## Phases

### Phase 1 — Tokens & textures (SC2, SC3)

Copy the 6 token CSS files + `base.css` texture utilities into `remote-frontend/src/styles/tokens/`, wire them into the existing `remote-frontend/src/index.css` entry, add a token-resolution unit test, and add a texture-utility DOM test. The node frontend's partial token set is NOT reconciled (out of scope, gap-analysis B-22; SC9). Light-mode opt-in via `data-theme` is included.

| ID  | Title                                  | SCs    | dep:                 | conflicts: |
| ---: | -------------------------------------- | ------ | -------------------- | ---------- |
| 101 | Port color + typography tokens          | SC2    | dep: -               | conflicts: - |
| 102 | Port spacing + radius + motion tokens | SC2    | dep: 101             | conflicts: - |
| 103 | Port fonts @import + base element CSS  | SC2    | dep: 101             | conflicts: - |
| 104 | Port texture utilities                 | SC3    | dep: 103             | conflicts: - |
| 105 | Wire tokens into index.css entry       | SC2    | dep: 101 102 103 104 | conflicts: - |
| 106 | Token + texture unit tests             | SC2 SC3| dep: 105             | conflicts: - |

### Phase 2 — Core + board + settings components (SC1, SC4, SC5, SC6)

Port the `.vks-*` component classes (190 lines) and the 14 React components (9 core + 3 board + 2 settings) from the design source into `remote-frontend/src/components/{core,board,settings}/`. Each component is a TypeScript port of its JSX sibling, using `cn()` for class composition and the `.d.ts` sibling for prop types. The component classes are copied byte-identical into `remote-frontend/src/styles/components.css`. A per-component unit test asserts the class names and prop-driven variant rendering.

| ID  | Title                                  | SCs              | dep:                | conflicts: |
| ---: | -------------------------------------- | ---------------- | ------------------- | ---------- |
| 201 | Port component CSS classes             | SC1              | dep: 105            | conflicts: - |
| 202 | Port Button + Badge + Card             | SC4              | dep: 201            | conflicts: - |
| 203 | Port Input + Switch + Checkbox         | SC4              | dep: 202            | conflicts: - |
| 204 | Port Tabs + Select + Loader            | SC4              | dep: 202            | conflicts: - |
| 205 | Port StatusBadge + TaskCard            | SC5              | dep: 201 202        | conflicts: - |
| 206 | Port NodeCard                          | SC5              | dep: 205            | conflicts: - |
| 207 | Port SettingsSection + SettingsRow     | SC6              | dep: 202            | conflicts: - |
| 208 | Component render + parity tests        | SC1 SC4 SC5 SC6  | dep: 202 203 204 205 206 207 | conflicts: - |

### Phase 3 — App UI kit + shell integration (SC7, SC8, SC9)

Port the app UI kit (`BoardView`, `Chrome`, `Panels`/`TaskDrawer`) into `remote-frontend/src/ui/`, wire them to the live hive REST + Electric data planes, replace the slim task-104 `NormalLayout`/`Navbar`/`BottomNav` with `Chrome`, and swap the `/nodes` + `/tasks` route placeholders for `NodesView` and `BoardView`/`TaskDrawer`. Add hive REST clients for nodes/tasks/labels against `/v1/*` (bare JSON, no envelope — established contract from `vk-swarm-hive-ui` tasks 102/307). Add Electric collections for the 3 synced task tables. The node frontend stays byte-for-byte unchanged (SC9 enforced by chained typecheck in every Done-when gate).

| ID  | Title                                  | SCs              | dep:                | conflicts: |
| ---: | -------------------------------------- | ---------------- | ------------------- | ---------- |
| 301 | Port BoardView + ColumnHeader          | SC7              | dep: 205 206        | conflicts: - |
| 302 | Port Chrome (Navbar/Logo/ThemeToggle)  | SC7              | dep: 202 207        | conflicts: - |
| 303 | Port Panels (NodesView/ProcessesView)  | SC7              | dep: 206 202        | conflicts: - |
| 304 | Port TaskDrawer + Diff/Logs/Attempts   | SC7              | dep: 202 204 205 302| conflicts: - |
| 305 | Hive REST clients (nodes/tasks/labels) | SC8              | dep: 102            | conflicts: - |
| 306 | Electric collections for task tables   | SC8              | dep: 106 201        | conflicts: - |
| 307 | Integrate Chrome into AppRouter        | SC8              | dep: 302 105        | conflicts: - |
| 308 | Wire BoardView + TaskDrawer to data    | SC8              | dep: 301 304 305 306 307 | conflicts: - |
| 309 | Wire NodesView + ProcessesView to data | SC8              | dep: 303 305 307    | conflicts: - |
| 310 | App integration + reachability gate    | SC7 SC8 SC9      | dep: 307 308 309    | conflicts: - |

## Gate (per AGENTS.md, supplemental remote-frontend checks)

```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
# supplemental (remote-frontend — no Rust, no frontend/ edits)
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
cd remote-frontend && npx vitest run
```

## Post-phase integrated adversarial review (per AGENTS.md §44-53)

After each phase, dispatch a cross-model adversarial review over the full phase diff. Report at `.agents/reports/YYYY-MM-DD-round-N-<panelist>-<2-word-description>.md`. Findings are remediated in-session (No Deferred Remediation). The next phase depends on the prior phase's integrated review passing.