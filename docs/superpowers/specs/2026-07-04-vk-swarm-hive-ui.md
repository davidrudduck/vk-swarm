---
doc_type: spec
status: draft
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