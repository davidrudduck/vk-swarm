# Plan — vk-swarm-hive-ui

> Spec: `docs/superpowers/specs/2026-07-04-vk-swarm-hive-ui.md` (frozen at precheck,
> `spec_sha=380f469b5626845e75129f7a2f7be44c5a7a0027`). Workstream tracker:
> `dev-docs/workstreams/vk-swarm-hive-ui/README.md`.

## Approach

Three independent tracks, each shippable on its own, composed into the hive console. The
spec's `## Approach` names them: (1) **app shell** — port the node frontend's session/auth
model + layout into `remote-frontend/`, replacing the 4-page auth stub; (2) **rehost** — copy
`frontend/src/components/swarm/*` into the hive shell (node frontend kept as HA fallback);
(3) **rewire** — add the 3 missing Electric collections + types and build the cross-node
aggregation board consuming them, with writes via the hive's existing REST routes.

Track 1 (shell) unblocks tracks 2 and 3 — without a shell there is nowhere to mount the
rehosted components or the cross-node views. Tracks 2 and 3 can proceed in parallel once track
1 lands the shell skeleton. Phases below are ordered: Phase 1 = shell; Phase 2 = rehost
(depends on 1); Phase 3 = rewire + cross-node board (depends on 1; can overlap with 2).

No server-side changes (all routes exist: `/oauth/web/*`, `/profile`, `/tasks/*`,
`/api/electric/v1/shape`, `/api/nodes`). No node-frontend changes (swarm components stay as HA
fallback). No new push channels (Electric polling only). No irreversible decisions (all four
design decisions are walkable; no ADRs).

## Phases

| Phase | Name | SCs | Deps |
|-------|------|-----|------|
| 1 | app-shell | SC3 | - |
| 2 | rehost-swarm | SC1, SC4 | phase 1 |
| 3 | rewire-cross-node | SC2, SC5 | phase 1 |

### Phase 1 — app-shell (SC3: net-new app shell replacing the 4-page auth stub)

Port `ConfigProvider` + `useAuth` + `oauthApi` (repointed at hive `/oauth/web/*` + `/profile`) +
`NormalLayout` (Navbar/BottomNav/Outlet) from `frontend/` into `remote-frontend/`. Replace the
4-route auth stub (`AppRouter.tsx`, `App.tsx`) with the full console route tree. This phase
delivers the skeleton every other phase mounts into.

Tasks:
| id | title | dep: | conflicts: |
|----|-------|------|------------|
| 100 | Install remote-frontend toolchain (vitest, eslint, @tanstack/react-db, testing-library) | dep: - | conflicts: none |
| 101 | Port ConfigProvider + config API client into remote-frontend | dep: 100 | conflicts: none |
| 102 | Port oauthApi client, repointed at hive /oauth/web/* | dep: 100 | conflicts: none |
| 103 | Port useAuth hook into remote-frontend | dep: 101 | conflicts: none |
| 104 | Port NormalLayout (Navbar + BottomNav + Outlet) into remote-frontend | dep: 101 | conflicts: none |
| 105 | Replace AppRouter with full console route tree (auth + console + fallback) | dep: 101 102 103 104 | conflicts: none |
| 106 | Wire ConfigProvider + QueryClientProvider into App.tsx root | dep: 101 102 105 | conflicts: none |

### Phase 2 — rehost-swarm (SC1: parity with node-frontend swarm UI; SC4: node frontend untouched)

Copy `frontend/src/components/swarm/*` and the supporting API clients (`nodesApi`,
`swarmProjectsApi`, `swarmLabelsApi`, `swarmTemplatesApi`) into `remote-frontend/`, mounting the
`Nodes.tsx` management page into the new shell. The node frontend keeps its copy (HA fallback).

Tasks:
| id | title | dep: | conflicts: |
|----|-------|------|------------|
| 201 | Rehost setup: add @/* + shared/* path aliases + copy shared types into remote-frontend | dep: 106 | conflicts: none |
| 202 | Copy swarm components tree into remote-frontend/src/components/swarm/ | dep: 201 | conflicts: none |
| 203 | Mount Nodes.tsx management page at /nodes in the hive shell | dep: 202 | conflicts: none |
| 204 | Parity smoke: every rehosted swarm component renders in the hive shell | dep: 203 | conflicts: none |

### Phase 3 — rewire-cross-node (SC2: cross-node aggregation board; SC5: rewired to Electric shapes)

Add the 3 missing Electric collections + types (`node_task_assignments`,
`node_task_output_logs`, `node_task_progress_events`) and build the cross-node tasks board
reading from them via the org-scoped Electric proxy, with management actions via the hive REST
routes. No new WebSocket/SSE.

Tasks:
| id | title | dep: | conflicts: |
|----|-------|------|------------|
| 300 | Copy frontend/src/lib/electric/ into remote-frontend/src/lib/electric/ (alias bridge for @/lib/electric) | dep: 106 | conflicts: none |
| 301 | Add ElectricTaskAssignment type + createTaskAssignmentsCollection | dep: 300 | conflicts: 302 303 |
| 302 | Add ElectricTaskOutputLog type + createTaskOutputLogsCollection | dep: 300 | conflicts: 301 303 |
| 303 | Add ElectricTaskProgressEvent type + createTaskProgressEventsCollection | dep: 300 | conflicts: 301 302 |
| 304 | Export the 3 new collections + types from electric/index.ts | dep: 301 302 303 | conflicts: none |
| 305 | Build TasksBoard page: cross-node kanban grouped by status | dep: 304 | conflicts: none |
| 306 | Build TaskDetail panel: attempts (output logs) + progress events drill-down | dep: 304 305 | conflicts: none |
| 307 | Wire management actions (set executing node / delete) to hive REST routes | dep: 305 | conflicts: 306 |
| 308 | No-push invariant: grep board + detail for WebSocket/EventSource/SSE (must be empty) | dep: 305 306 307 | conflicts: none |

## SC → task coverage

- **SC1** (parity with node-frontend swarm UI): 201, 202, 203, 204
- **SC2** (cross-node aggregation views): 305, 306
- **SC3** (net-new app shell): 100, 101, 102, 103, 104, 105, 106
- **SC4** (node frontend untouched / HA fallback): 202 (copy, not move — verified by 204 parity + node-frontend lint/tsc staying green)
- **SC5** (rewired to Electric shapes, no new push): 300, 301, 302, 303, 304, 308

## TS coverage

- **TS1** (app shell unit): 101, 102, 103
- **TS2** (app shell integration/router): 105, 106
- **TS3** (rehost smoke/parity): 204
- **TS4** (rewire unit — collections/types): 301, 302, 303, 304
- **TS5** (rewire integration — board renders across nodes): 305, 306
- **TS6** (mutation — assign from board): 307
- **TS7** (no-push invariant): 308

## Post-phase integrated adversarial review (per AGENTS.md)

Per AGENTS.md, after completing each WAI phase, run an **integrated adversarial review**
(Gemini or cross-model) over the full phase diff before moving to the next phase. Findings
are subject to the No Deferred Remediation rule: fix in-session or dismiss with ledger
evidence. This is an execute-time gate (not a decompose artifact) — `/wai:execute` runs it
after each phase's tasks pass their individual gates. Reports go to
`.agents/reports/YYYY-MM-DD-round-N-<panelist>-<2-word-description>.md`.

## Gate (per AGENTS.md — mandatory, unchanged)

The AGENTS.md finish-gate runs on the final committed state — do NOT rewrite or drop any of
these four commands. This workstream touches only TypeScript (`remote-frontend/` and, for the
rewire collections, `frontend/src/lib/electric/`), so the Rust gates are unchanged-Rust
regression guards, not the workstream's own gate:

```bash
cargo clippy --all --all-targets --all-features -- -D warnings   # unchanged Rust, must stay green
cargo test --workspace                                            # unchanged Rust
cd frontend && npm run lint                                       # node frontend (HA fallback) untouched
cd frontend && npx tsc --noEmit                                    # node frontend typecheck
```

Supplemental workstream-specific gates (run AFTER the four AGENTS.md commands above):

```bash
cd remote-frontend && npm run lint                                # eslint --max-warnings 0 on the hive frontend
cd remote-frontend && npx tsc --noEmit                            # hive frontend typecheck
cd remote-frontend && npx vitest run                              # hive frontend test suite
```

The AGENTS.md four are the mandatory finish-gate; the supplemental three confirm the hive
frontend (the workstream's target) compiles + lints + tests green. Both sets must pass before
the PR is pushed.