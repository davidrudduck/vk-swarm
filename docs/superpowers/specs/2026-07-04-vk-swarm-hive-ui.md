---
doc_type: spec
status: active
workstream: vk-swarm-hive-ui
change_kind: behaviour
---

# vk-swarm-hive-ui — Hive central-management web UI (SC1 UI half)

> **Child of** [`vk-swarm-refactor`](./2026-06-25-vk-swarm-refactor.md). **Depends on**
> [`vk-swarm-hive-redesign`](../../dev-docs/workstreams/vk-swarm-hive-redesign/spec/2026-06-26-vk-swarm-hive-redesign.md)
> — the protocol + data plane (SC2–SC7 and SC1's no-fan-out, data-plane half) is shipped (PRs #450,
> #451). This workstream builds the **central console on top of it**.
>
> **Carve record:** split out of `vk-swarm-hive-redesign` SC1 by user decision (2026-06-30) when
> decompose found the "hive web UI manages all" half of SC1 is a workstream-scale frontend effort
> (a *rehost + rewire + net-new cross-node views* build), distinct from the protocol rebuild. The
> carve is recorded in the hive-redesign decisions ledger; no frozen-spec edit was made.

## Intent (what / why)

The hive is the **central management plane** (per `vk-swarm-hive-redesign` SC1), but today it serves
only a 4-page auth stub in `remote-frontend/` (`HomePage`, `InvitationPage`,
`InvitationCompletePage`, `NotFoundPage` — `AppRouter.tsx`, 4 routes). Its API client calls only
`/v1/invitations/*` and `/api/oauth/web/*`. **No nodes dashboard, no board, no task/attempt/execution
views are hosted by the hive.** The SPA is served as a static fallback
(`crates/remote/src/routes/mod.rs:107-115`).

Meanwhile the swarm-management UI **already exists but lives in the NODE frontend**
(`frontend/src/components/swarm/*`, `frontend/src/pages/Nodes.tsx`), reaching hive-only endpoints
(`/api/electric/v1/shape`, `/api/nodes`) that have no node-server handler — so it is already designed
to be hive-backed. Today it manages: nodes, swarm projects, node↔project links, labels, templates,
API keys, health. **NOT managed anywhere:** cross-node tasks/attempts/executions (grep of
`frontend/src/components/swarm` + `lib/electric` for `task_attempts`/`execution_process` is empty).
The hive already **publishes** `node_task_assignments` / `node_task_output_logs` /
`node_task_progress_events` Electric shapes (`frontend/src/lib/electric/config.ts:73-89`) with no
consumer yet.

This workstream builds the hive-hosted central console by **rehosting** the existing swarm components
from the node frontend into the hive app shell, **rewiring** task/attempt/execution component trees
at the already-published cross-node Electric shapes, and building the **net-new cross-node
aggregation views** (tasks/attempts/executions across all nodes in one board) plus the hive-hosted
**app shell** (nav, layout, session-as-app auth) that `remote-frontend/` has none of.

## Users / who is affected

- **Operator/admin (browser):** the human who manages nodes, swarm projects, node↔project links,
  labels, templates, API keys, and health from a web console. Today this persona uses the
  node-frontend swarm components (hosted on a node); after this workstream they use the hive-hosted
  console as the primary surface.
- **End-user / coder (browser):** the human running coding agents on tasks. After this workstream
  they view tasks/attempts/executions **across all nodes** in one hive-hosted board, rather than
  switching between per-node local UIs. Per-task execution detail remains viewable on the node's
  always-on local UI (the HA fallback).
- **Node-frontend swarm components:** kept as the **HA fallback** during hive outage (consistent
  with the hive-redesign HA constraint, spec §"Out of scope"). The hive UI is primary; the node UI
  is secondary. The node frontend's local-task UI is untouched; only swarm-management components
  are rehosted/rewired (not deleted from the node frontend).

## User stories

- **US1:** As an operator, when I open the hive console, I expect to see and manage all connected
  nodes, projects, node↔project links, labels, templates, API keys, and health — the same
  management surface that today lives in the node-frontend swarm UI.
- **US2:** As an operator/end-user, when I open the hive console, I expect to see tasks, attempts,
  and executions **across all nodes** in one aggregated board — not switch between per-node UIs.
- **US3:** As an operator, when the hive is temporarily down, I expect the node-frontend swarm UI
  to keep working as a fallback (no regression from today's behaviour).
- **US4:** As an end-user, when I want per-task execution detail (logs, streaming), I expect to
  reach it from the hive console's task view, with the option to drop into the node's local UI for
  the live execution surface.
- **US5:** As an operator, when I sign into the hive console, I expect a session-as-app auth flow
  consistent with the node frontend (nav, layout, persisted session), not the current 4-page auth
  stub.

## Success criteria

- **SC1:** The hive-hosted console renders every management view that exists today in
  `frontend/src/components/swarm/*` (nodes, swarm projects, node-projects, labels, templates, API
  keys, health) — **parity** with the node-frontend swarm UI, served from the hive app shell.
  → US1
- **SC2:** The hive-hosted console renders **cross-node aggregation views** for tasks, attempts,
  and executions across all connected nodes in one board, reading from the already-published
  Electric shapes (`node_task_assignments` / `node_task_output_logs` /
  `node_task_progress_events`, `frontend/src/lib/electric/config.ts:73-89`) via the org-scoped
  Electric proxy (`crates/remote/src/routes/electric_proxy.rs`). → US2
- **SC3:** The hive-hosted console has a **net-new app shell** — nav, layout, and session-as-app
  auth reused from the node frontend's session/auth model — replacing the current 4-page auth stub
  in `remote-frontend/`. → US5
- **SC4:** The node-frontend swarm UI (`frontend/src/components/swarm/*`,
  `frontend/src/pages/Nodes.tsx`) **continues to render and function** as the HA fallback; no
  regression from today's behaviour (the rehost reuses, not deletes, these components). → US3
- **SC5:** The hive console's task/attempt/execution component trees are **rewired** to consume the
  already-published cross-node Electric shapes (add collections + consumers in
  `frontend/src/lib/electric/`); no new WebSocket/SSE push channels are introduced (the UI reads
  Postgres-direct via the Electric proxy). → US2

## Constraints

- **Keep:** Postgres as the hive store (read via the org-scoped Electric proxy), the
  Electric-over-Postgres client (`collections.ts`, `config.ts`), the existing
  `frontend/src/components/swarm/*` component tree (reuse as-is), and the existing org-scoped
  Electric proxy (`crates/remote/src/routes/electric_proxy.rs`).
- **Reuse node-frontend auth:** the hive app shell's session/auth model is ported from the node
  frontend (consistency across surfaces). This is a **candidate irreversible decision** — flagged
  for an ADR in `/wai:spec` if porting vs extending hive auth proves architecturally entangled.
- **No new push channels:** the UI reads from the already-published Electric shapes
  (Postgres-direct via Electric proxy). No new WebSocket/SSE push paths.
- **Node frontend is HA fallback, not deleted:** swarm-management components stay in the node
  frontend as the always-on fallback during hive outage (matches the hive-redesign HA constraint).
  The node frontend's local-task UI is untouched.
- **Start only after `vk-swarm-hive-redesign` is shipped** — ✓ verified (PRs #450/#451 merged, all
  gates green). The protocol/data plane this console renders on top of is in place.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **Hive HA / multi-instance** — the hive UI runs on a single hive instance; HA/replication is a
  future workstream (consistent with the hive-redesign out-of-scope).
- **Node-local durability and crash-resumability** — owned by `vk-swarm-node-foundations`.
- **Schema/migration consolidation** beyond what the rehost/rewire requires.
- **P3+ AI breakdown / event bus / management agent** — consumers of this plane, designed later.
- **Mobile / responsive design** — desktop browser only; no mobile or responsive design work.
- **Node-local UI changes** — the node frontend's local-task UI is untouched; only
  swarm-management components are rehosted/rewired.
- **Real-time push beyond Electric** — no new WebSocket/SSE push channels; the UI reads from the
  already-published Electric shapes.
- **Protocol / data-plane work** — owned by `vk-swarm-hive-redesign` (shipped). This workstream
  consumes its published shapes; it does not alter the protocol.

## Approach

Three parallel tracks, each independently shippable, composed into the hive console:

1. **App shell (net-new):** port the node frontend's session/auth model (`ConfigProvider`,
   `useAuth`, `oauthApi` client pattern, `NormalLayout` shell with Navbar/BottomNav/Outlet) into
   `remote-frontend/`, repointing the API calls at the hive's already-existing
   `/oauth/web/*` and `/profile` routes. Replace the 4-page auth stub (`AppRouter.tsx`,
   `App.tsx`). This unblocks the other two tracks — without a shell there is nowhere to mount
   the rehosted components or the cross-node views.

2. **Rehost (copy, not move):** copy `frontend/src/components/swarm/*` into
   `remote-frontend/src/components/swarm/*`. The node frontend keeps its copy (HA fallback,
   SC4). The components are self-contained React + `@tanstack/react-query` and already target
   hive endpoints (`/api/nodes`, `/api/electric/v1/shape`), so the copy mounts into the new
   shell with minimal adaptation (router wiring, shared `QueryClientProvider`,
   `ConfigProvider` context).

3. **Rewire + cross-node views (net-new):** add the 3 missing Electric collections
   (`node_task_assignments`, `node_task_output_logs`, `node_task_progress_events`) and their
   types to `frontend/src/lib/electric/`, then build the cross-node aggregation board
   (tasks/attempts/executions across all nodes) consuming those collections via the org-scoped
   Electric proxy. Management actions (assign, delete, reassign) use the hive's existing REST
   routes (`/tasks/{id}/assign`, `/tasks/{id}` DELETE/PATCH) — reads via Electric, writes via
   REST.

Tracks compose: track 1 delivers the shell; track 2 mounts the management surface into the
shell; track 3 mounts the cross-node board into the shell. Tracks 2 and 3 can proceed in
parallel once track 1 lands the shell skeleton.

## Design / architecture

### App shell (track 1)

```
remote-frontend/src/
├── App.tsx                 # was: placeholder. now: <ConfigProvider><AppRouter/></ConfigProvider>
├── AppRouter.tsx           # was: 4 auth routes. now: full route tree (console + auth fallback)
├── components/
│   ├── ConfigProvider.tsx  # ported from frontend/, repointed at hive /profile + /oauth/web/*
│   └── layout/
│       └── NormalLayout.tsx  # ported from frontend/ (Navbar + Outlet + BottomNav)
├── hooks/
│   └── auth/
│       └── useAuth.ts      # ported from frontend/
├── lib/
│   └── api/
│       ├── oauth.ts        # ported from frontend/, repointed: /api/auth/* → /oauth/web/*
│       └── config.ts       # ported from frontend/ (fetches hive /profile as UserSystemInfo)
└── pages/                  # net-new console pages
    ├── Nodes.tsx           # rehosted management surface
    ├── Tasks.tsx           # cross-node board (track 3)
    └── ...
```

**Auth flow:** `ConfigProvider` fetches `GET /profile` on mount → sets `loginStatus`. If
unauthenticated, the router redirects to the OAuth provider picker → `POST /oauth/web/init`
(PKCE) → redirect to provider → `GET /oauth/{provider}/callback` → `POST /oauth/web/redeem`
→ session cookie established → redirect back to console. Logout via `POST /oauth/logout`.

**Key repoint:** the node frontend's `oauthApi` calls `/api/auth/handoff/init` (node server
proxies to hive). The hive `remote-frontend/` calls `/oauth/web/init` directly — the hive
server's `oauth.rs` `public_router()` already exposes these. No server changes needed.

**What stays:** `pkce.ts` (already in `remote-frontend/`), `api.ts` invitation/handoff types
(absorbed into the new `lib/api/oauth.ts`), Tailwind/Vite/tsconfig setup.

### Rehost (track 2)

Copy `frontend/src/components/swarm/*` → `remote-frontend/src/components/swarm/*`. The
components use `@tanstack/react-query` with API clients that already target hive endpoints
(`/api/nodes`, `/api/swarm-projects`, etc. via the node server's proxy). In the hive app shell
these clients call the hive directly — the `QueryClientProvider` and `ConfigProvider` contexts
are ported in track 1, so the components mount with context wiring only.

**API client porting:** the node frontend's `lib/api/` clients (`nodesApi`, `swarmProjectsApi`,
`swarmLabelsApi`, `swarmTemplatesApi`) are copied into `remote-frontend/src/lib/api/` with
base URLs adjusted to call the hive directly (no `/api` proxy prefix if the hive serves these
at the same paths, or with the hive's route prefixes per `crates/remote/src/routes/`).

**Router wiring:** `Nodes.tsx` (the swarm management page) mounts at `/nodes` in the new
`AppRouter.tsx`. The component's internal tabs (health, projects, labels, templates) are
unchanged.

### Rewire + cross-node views (track 3)

**New Electric collections** in `frontend/src/lib/electric/collections.ts`:

- `createTaskAssignmentsCollection()` — `node_task_assignments` table
- `createTaskOutputLogsCollection()` — `node_task_output_logs` table
- `createTaskProgressEventsCollection()` — `node_task_progress_events` table

Each uses the same `electricCollectionOptions` pattern as the existing 3 collections
(`createNodesCollection`, `createProjectsCollection`, `createNodeProjectsCollection`),
pointing at `ELECTRIC_PROXY_BASE` with the table name from `ELECTRIC_SHAPE_TABLES`.

**New types** in `collections.ts`:

- `ElectricTaskAssignment` (id, task_id, node_id, lease_owner, fencing_token, status, assigned_at, leased_until)
- `ElectricTaskOutputLog` (id, task_id, node_id, entry_index, content, timestamp, stream)
- `ElectricTaskProgressEvent` (id, task_id, node_id, event_type, payload, timestamp)

**Cross-node board** (`remote-frontend/src/pages/Tasks.tsx`):

- Reads from the 3 new collections + the existing `projects` and `nodes` collections for
  join/enrichment (task → project name, task → node name).
- Groups tasks by status (todo/in-progress/blocked/done) across all nodes — a kanban board.
- Per-task drill-down: attempts (from `node_task_output_logs`) and execution events (from
  `node_task_progress_events`) in a detail panel.
- Management actions (assign, reassign, delete) call the hive REST routes:
  `POST /tasks/{id}/assign`, `PATCH /tasks/{id}/executing-node`, `DELETE /tasks/{id}`.

**No new push channels:** the Electric proxy polls Postgres via shape URLs. The UI reactively
updates via `@tanstack/react-db` collection change events. No WebSocket/SSE.

### What is NOT built

- No server-side changes (all routes exist: `/oauth/web/*`, `/profile`, `/tasks/*`,
  `/api/electric/v1/shape`, `/api/nodes`).
- No node-frontend changes (swarm components stay as HA fallback).
- No mobile/responsive design (desktop browser only).
- No real-time push beyond Electric's Postgres shape polling.

## Decisions

1. **Port node-frontend auth pattern into hive app shell** (ConfigProvider + useAuth + oauthApi
   → hive `/oauth/web/*` + `/profile`). The hive server already exposes equivalent routes; this
   is UI pattern reuse, not a wire-format or contract change. **Reversible:** swap the
   `ConfigProvider` for a different session model without touching the server. Not irreversible
   — no ADR required. The spec's Constraints section flagged this as a candidate irreversible
   decision; on design investigation it is walkable.

2. **Copy swarm components (not extract to shared package).** The node frontend and hive
   `remote-frontend/` are separate Vite apps. A shared package would require workspace
   publishing or monorepo tooling. Copying is pragmatic: the components are stable
   (management UI, not rapidly evolving), the node frontend's copy is the HA fallback (frozen
   unless the hive is down), and drift can be reconciled by extracting to a shared package
   later if it becomes painful. **Reversible:** extract to `packages/swarm-ui/` later without
   breaking either app. Not irreversible — no ADR required.

3. **Reads via Electric collections, writes via REST.** The cross-node board reads from
   Electric shapes (reactive, Postgres-direct via proxy) and writes management actions via the
   hive's REST routes (`/tasks/{id}/assign`, etc.). This split matches the hive-redesign data
   plane: Electric for reads (shape-based, org-scoped), REST for mutations (fenced, lease
   checked server-side). **Reversible:** the board's data layer is UI-internal; switching to
   REST reads would rewrite the collection hooks, not a contract change. Not irreversible — no
   ADR required.

4. **No new push channels (Electric polling only).** The UI relies on Electric's Postgres shape
   polling for reactivity. No WebSocket/SSE. This is a constraint from the spec, restated as a
   design decision: the shape polling cadence is sufficient for a management console (not a
   live-tail terminal). **Reversible:** add a WS/SSE channel later for lower latency without
   breaking the Electric reads. Not irreversible — no ADR required.

> **No irreversible decisions (delete/migrate/rename/breaking/wire-format) are introduced by
> this workstream.** All four decisions are walkable. The spec's Constraints section flagged
> the auth-port as a candidate; on investigation it is pattern reuse against existing server
> routes, not a contract change. `/wai:precheck` should pass assert 2 (ADRs for irreversible
> decisions) with zero new ADRs.

## Test strategy

### App shell (track 1)

- **Unit:** `ConfigProvider` fetches `/profile`, sets `loginStatus` correctly for
  authenticated/unauthenticated/error states. `useAuth` exposes `isSignedIn`/`isLoaded`/`userId`
  derived from context. `oauthApi` client calls the right hive endpoints
  (`/oauth/web/init`, `/oauth/web/redeem`, `/oauth/logout`, `/profile`).
- **Integration (router):** unauthenticated user hitting `/nodes` → redirected to OAuth
  provider picker → (mocked) callback → session established → landed on `/nodes`.
  Logout → session cleared → redirected to login.
- **E2E (Playwright, if feasible in CI):** full OAuth round-trip against a local hive with
  a mock provider.

### Rehost (track 2)

- **Smoke:** each rehosted swarm component (NodeCard, SwarmProjectsSection, SwarmLabelsSection,
  SwarmTemplatesSection, SwarmHealthSection) renders in the hive shell with mocked
  `QueryClientProvider` data. No missing-context errors.
- **Parity:** visual snapshot or DOM-structure parity between the node-frontend swarm page
  and the hive-frontend rehosted page (same component tree, same data shape).

### Rewire + cross-node views (track 3)

- **Unit:** the 3 new Electric collections (`createTaskAssignmentsCollection`,
  `createTaskOutputLogsCollection`, `createTaskProgressEventsCollection`) produce correct
  shape URLs (table name from `ELECTRIC_SHAPE_TABLES`, base from `ELECTRIC_PROXY_BASE`).
  Types (`ElectricTaskAssignment`, `ElectricTaskOutputLog`, `ElectricTaskProgressEvent`)
  match the shape columns.
- **Integration:** the cross-node board renders tasks grouped by status across multiple mocked
  nodes. Task drill-down shows attempts (from `node_task_output_logs`) and progress events
  (from `node_task_progress_events`).
- **Mutation:** `POST /tasks/{id}/assign` from the board → task moves to the assigned node's
  column → collection updates reactively.
- **No-push invariant:** grep the new board code for `WebSocket`/`EventSource`/`SSE` — must be
  empty (enforces the no-new-push-channels constraint).

### Gate checks (per AGENTS.md)

```bash
cargo clippy --all --all-targets --all-features -- -D warnings   # no server changes, but gate stays green
cargo test --workspace                                            # no server changes
cd remote-frontend && npm run lint                                # eslint --max-warnings 0
cd remote-frontend && npx tsc --noEmit                            # typecheck the hive frontend
```

The hive frontend (`remote-frontend/`) has its own `package.json` and tsconfig — the lint and
tsc gates run there, not in the node `frontend/`. The Rust gates run on the workspace but
should be unchanged (no server code touched).