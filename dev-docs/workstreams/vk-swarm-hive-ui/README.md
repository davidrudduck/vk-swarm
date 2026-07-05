---
workstream: vk-swarm-hive-ui
doc_type: readme
status: shipped
title: "Hive central-management web UI — host the cross-node console (SC1 UI half)"
depends_on: [vk-swarm-hive-redesign]
adrs: []
staging_pointers:
  - docs/plans/vk-swarm-hive-ui
  - docs/superpowers/specs/2026-07-04-vk-swarm-hive-ui.md
---

# vk-swarm-hive-ui

**Carved out of `vk-swarm-hive-redesign` SC1 by user decision (2026-06-30)** when decompose found the
"hive web UI manages all" half of SC1 is a workstream-scale frontend effort (a *rehost + rewire + new
cross-node views* build), distinct from the protocol rebuild. `vk-swarm-hive-redesign` delivers the
**protocol + data plane** (SC2–SC7 and SC1's no-fan-out, data-plane half); this workstream builds the
central console on top of it.

No spec yet — this is a tracker stub for a future `/wai:prd-new` + `/wai:spec` + `/wai:precheck` +
`/wai:decompose`, sequenced AFTER `vk-swarm-hive-redesign`.

## Why this is a real build (grounded, not a re-skin)

Verified during the hive-redesign decompose (read-only investigation):

- **The hive serves only a 4-page auth stub today** — `remote-frontend/` has `HomePage` (a "return to
  the app" placeholder), `InvitationPage`, `InvitationCompletePage`, `NotFoundPage` (`AppRouter.tsx`,
  4 routes). Its API client calls only `/v1/invitations/*` and `/v1/oauth/web/*`. **No nodes dashboard,
  no board, no task/attempt/execution views are hosted by the hive.** The SPA is served as a static
  fallback (`crates/remote/src/routes/mod.rs:107-115`).
- **The swarm-management UI exists but lives in the NODE frontend** (`frontend/src/components/swarm/*`,
  `frontend/src/pages/Nodes.tsx`), reaching hive-only endpoints (`/api/electric/v1/shape`,
  `/api/nodes`) that have no node-server handler — so it is already designed to be hive-backed.
- **Today managed:** nodes, swarm projects, node↔project links, labels, templates, API keys, health.
  **NOT managed anywhere:** cross-node tasks/attempts/executions (grep of `frontend/src/components/swarm`
  + `lib/electric` for `task_attempts`/`execution_process` is empty). The hive already *publishes*
  `node_task_assignments`/`node_task_output_logs`/`node_task_progress_events` Electric shapes
  (`frontend/src/lib/electric/config.ts:73-89`) with no consumer yet.

## Scope (rehost + rewire + net-new)

- **Reuse as-is:** the `frontend/src/components/swarm/*` tree (nodes, swarm projects, node-projects,
  labels, templates, health), the Electric-over-Postgres client (`collections.ts`, `config.ts`), and
  the org-scoped Electric proxy (`crates/remote/src/routes/electric_proxy.rs`).
- **Rewire:** point task/attempt/execution component trees at the already-published cross-node Electric
  shapes (add collections + consumers).
- **Net-new (the real build):** a hive-hosted **app shell** (nav, session-as-app auth, layout —
  `remote-frontend/` has none) and **cross-node aggregation views** (tasks/attempts/executions across
  all nodes).

## Relationship to the program

Child of `vk-swarm-refactor`. Depends on `vk-swarm-hive-redesign` (the protocol/data plane it renders).
SC1 remains in the hive-redesign spec, claimed by that workstream's no-fan-out (data-plane) phase; this
workstream owns the UI half. No frozen-spec edit was made — the carve is recorded in the
hive-redesign decisions ledger.
