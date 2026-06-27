# ADR-0002 — Node web UI scoped to local-only (remove remote-state display)

- **Status:** accepted
- **Date:** 2026-06-26
- **Workstream:** vk-swarm-node-foundations

## Context

Today a node's web UI can display and manage **remote** (other-node) projects/tasks: the API mixes
local and hive state, and dedicated frontend surfaces (Nodes page, remote badges, merged-project
views, remote log/diff streaming) render the swarm. This multi-node display is part of the §2 sync
fragility surface and conflicts with the program's decided hub-and-spoke topology, where the **hive**
owns cross-node management and **nodes manage only their own local work** (umbrella constraint;
`vk-swarm-node-foundations` SC5). The inbound sync writers, the outbound publisher, the WS node runner,
and the `remote_*` / `shared_task_id` columns must remain intact — `vk-swarm-hive-redesign` depends on
them.

## Decision

Scope the node UI to **local work + read-only hive-sync visibility**, changing only the **read/API
layer and the frontend** — never the sync plumbing:

- **Visibility discriminator (intent):** a task/project is node-visible iff it was **created on this
  node** *or* **has a local `task_attempt`** (i.e. this node is/was running it). The naive
  `shared_task_id IS NULL` filter is rejected — it would hide hive-*assigned* work the node is actively
  executing. The exact SQL predicate is confirmed at decompose against how assignment populates rows.
- **API:** apply the discriminator in `Task::find_by_project_id_with_attempt_status`
  (`crates/db/src/models/task/queries.rs:15`) and **remove the request-time remote merge** in
  `get_tasks` (`crates/server/src/routes/tasks/handlers/core.rs:97-173`). Remove/relocate purely-remote
  proxies (`/api/nodes*`, `/api/swarm/*`, `/api/merged-projects`, available-nodes/stream-connection-info)
  from the node surface.
- **Frontend (delete):** the Nodes feature (`pages/Nodes.tsx`, `components/nodes/*`, `NodesContext`,
  `lib/api/nodes.ts`, navbar entry), remote badges on local cards, `useMergedProjects`, and remote
  log/diff/connection-status hooks.
- **Keep, read-only:** a hive sync status/config view built on `GET /api/database/sync-status`
  (`crates/server/src/routes/database.rs:295`), extended with `hive_url`/`node_name`/last-synced;
  `SwarmSettings` repurposed read-only.

## Consequences

- This **deletes frontend code** and **changes the `get_tasks` response shape** (no remote rows) — an
  API-contract change, hence this ADR.
- Inbound sync still writes remote rows into the local `tasks` table; we **filter at read**, we do
  **not** delete rows or clear `shared_task_id`/`remote_*` columns — so `vk-swarm-hive-redesign` retains
  its data and the outbound publisher/dedup keep working.
- The node becomes simpler and self-contained; cross-node management moves wholesale to the future hive.

## Alternatives considered

- **Hide remote in UI but keep the API merged** — rejected: leaves the fragile merge path and leaks
  remote state through the API.
- **Delete remote rows / clear columns on the node** — rejected: breaks the outbound publisher and
  destroys data the hive-redesign needs.
- **A headless/no-UI node build** (the original "light node" idea) — superseded: the node **always**
  serves its (scoped) local UI; no headless flag is introduced.
