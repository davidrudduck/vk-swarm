---
doc_type: spec
status: draft
workstream: vk-swarm-node-ui-localize
change_kind: bugfix
verify_cmd: "curl -fsS http://127.0.0.1:${BACKEND_PORT:-3001}/api/nodes?organization_id=$VK_ORG_ID | grep -q '\"success\":true'"
---

# vk-swarm-node-ui-localize — Intent

Finishes the React-layer half of the node/hive split that `vk-swarm-node-foundations` started.
node-foundations delivered the local-only backend core and deleted the node-surface API route
layer; the node frontend was never updated to match, so parts of it now call HTTP routes that no
longer exist.

Supersedes the "Scope (the entangled remainder)" and "Keep (do NOT remove)" lists in
`dev-docs/workstreams/vk-swarm-node-ui-localize/README.md`, which were written during
node-foundations decompose and have since drifted from merged code (see Constraint C1).

## Intent (what / why)

Two things are wrong in the node frontend today, and they pull in opposite directions:

**1. The swarm/nodes management UI is live but broken.** Its pages render and then every data
call 404s:

| Route | Component | Calls | Status |
|---|---|---|---|
| `/nodes` (`App.tsx:153`) | `pages/Nodes.tsx` | `nodesApi.list` → `/api/nodes` | 404 |
| `/settings/swarm` (`App.tsx:189`) | `SwarmProjectsSection` | `/api/swarm/projects` | 404 |
| ″ | `NodeProjectsSection` (`SwarmSettings.tsx:163`) | `/api/nodes`, `/api/nodes/{id}/projects` | 404 |
| ″ | `SwarmLabelsSection` | `/api/swarm/labels` | 404 |
| ″ | `SwarmTemplatesSection` | `/api/swarm/templates` | 404 |
| ″ | `NodeTemplatesSection` | `/api/swarm/templates`, `/api/nodes` | 404 |

`crates/server/src/routes/mod.rs:44-71` registers no `nodes` or `swarm` module. The org picker at
the top of `/settings/swarm` works — `/api/organizations` survived as a hive proxy — which makes
the failure look like empty data rather than a broken page.

The decision (2026-07-30) is to **repoint, not delete**: the node keeps its swarm management UI
and it talks to the hive. The route layer is restored as thin proxy handlers in the node server,
mirroring the pattern already live in `crates/server/src/routes/organizations.rs`. This is
tractable because node-foundations deleted only the HTTP layer — `RemoteClient`
(`crates/services/src/services/remote_client.rs`) still carries every method needed:
`list_nodes`, `get_node`, `delete_node`, `list_node_projects`, `list_linked_node_projects`,
`list_node_api_keys`, `create_node_api_key`, `revoke_node_api_key`, `get_node_statuses`, and the
full `*_swarm_project` / `*_swarm_label` / `*_swarm_template` sets including the merge and
`promote_label_to_swarm` operations.

**2. The task board still renders through the remote-merge types.** `useMergedProjects` →
`projectsApi.getMerged()` → `/api/merged-projects`, and `ProjectList` / `ProjectSwitcher` are
typed on `MergedProject`. The board should show **local projects only**; the merged view is
retired. This is the typed refactor node-foundations task 403 deliberately deferred to stay
codegen-neutral.

The distinction to hold onto: **management of the swarm is a hive concern the node proxies to;
display of the node's own work is local-only.** Those are not in conflict — they are the two
halves of the split.

## Users / who is affected

- **Node operators** managing swarm projects, labels, templates, and node-project links from a
  node's own UI. Today every one of those screens is silently empty.
- **Node operators pairing a node to a hive** via `/nodes` and the API-key surface — currently
  a dead page.
- **Anyone using the node task board**, who is served through a merge path the node no longer
  needs.
- **Future maintainers:** `frontend/src/components/swarm/` and
  `remote-frontend/src/components/swarm/` are near-identical 15-component trees. This workstream
  does not merge them, but the spec must say which is authoritative so the duplication is a
  known cost rather than an accident.

## Success criteria

Runtime-observable. No criterion is "test X passes".

- **SC1** — With a hive configured, `GET /api/nodes?organization_id=<org>` on a running node
  returns `200` with `success: true`, and the same for `/api/nodes/{id}`,
  `/api/nodes/{id}/projects`, `/api/nodes/api-keys`, `/api/swarm/projects`, `/api/swarm/labels`,
  `/api/swarm/templates`. Zero `404`s across the surface listed in the Intent table.
- **SC2** — Loading `/settings/swarm` in a browser against a hive-connected node renders real
  swarm projects, labels, and templates from the hive; the browser network log shows no `404`.
- **SC3** — Loading `/nodes` lists the organization's nodes with live status, and the API-key
  actions (create / revoke / unblock) round-trip against the hive.
- **SC4** — With **no** hive configured, every one of those screens renders an explicit
  "not connected to a hive" state. No unhandled rejection, no infinite spinner, no raw error
  body. (`RemoteClient` construction fails with `RemoteClientNotConfigured`; the UI must handle
  it as a first-class state.)
- **SC5** — The task board renders projects from `/api/projects`. `/api/merged-projects` receives
  zero requests from the frontend during a full board session (observable in the network log).
- **SC6** — Attempt logs, diffs, connection status, and node selection in the attempt UI behave
  correctly on a node with no hive: `ProcessLogsViewer`, `DiffsPanel`, `AttemptHeaderActions`,
  and `CreateAttemptDialog` render local state without erroring on absent remote data.
- **SC7** — `remote-frontend` behaviour is unchanged: the hive's own swarm UI still works
  end-to-end (its E2E suite is the regression signal).

## Constraints

- **C1 — The README's entanglement map is stale; re-derive it, do not trust it.** Its "Keep (do
  NOT remove — live non-Nodes consumers)" list asserts the Nodes feature was deleted by
  node-foundations. It was not: `/nodes` is still routed at `App.tsx:153`, and the "live
  consumers" it protects (`nodesApi`, `NodeProjectsSection`) are exactly the ones calling dead
  routes. Every claim in that section must be re-verified against merged code during
  `/wai:spec`. Recorded as a spec defect in the README and in `dev-docs/BACKLOG.md`.

- **C2 — Restoring node-surface proxies partially reverses ADR-0002 and needs its own ADR.**
  node-foundations removed the node-surface API proxies as an architectural decision. Re-adding
  them — even as thin `RemoteClient` pass-throughs — is a reversal of that decision and is
  irreversible in the ADR sense. `/wai:spec` must author an ADR covering: why proxy-through-node
  beats browser→hive direct, the auth boundary, and what stays deleted. Do not decompose without
  it.

- **C3 — The browser cannot hold hive credentials.** `VK_NODE_API_KEY` is a server-side secret.
  Proxying through the node server (the `organizations.rs` pattern) is what keeps it that way;
  any design that has the node frontend call the hive directly must solve browser-side auth and
  CORS, and should be rejected on that basis unless it can.

- **C4 — Hive-absent is a normal state, not an error path.** A node may run standalone. Every
  proxied surface must degrade to SC4's explicit disconnected state.

- **C5 — Type-generation discipline.** Retiring `MergedProject` touches a `#[ts(export)]` struct.
  `npm run generate-types` must be run and `npm run generate-types:check` must pass; no
  hand-edited `shared/types.ts`.

- **C6 — Full quality gates (CLAUDE.md "Finish What We Start").** `cargo clippy --all
  --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cd frontend && npm run
  lint`, `npx tsc --noEmit`, `cd remote-frontend && npm run lint && npx tsc --noEmit && npx
  vitest run` all green before the PR.

- **C7 — The two `components/swarm/` trees stay separate.** Unifying node and hive swarm
  components is explicitly not in scope; the spec records which is authoritative and the
  duplication is accepted for now.

## Out of scope

- Merging / deduplicating `frontend/src/components/swarm/` with
  `remote-frontend/src/components/swarm/` (C7).
- Any change to hive-side (`crates/remote/`) endpoints or `remote-frontend/`, beyond not
  regressing them (SC7).
- The hive UI findings **F-2026-07-29-03** (drawer/navbar actions blocked on hive APIs) and
  **F-2026-07-22-01** (design-system tokens) — different workstreams.
- **F-2026-07-29-04** (local vitest/vite-build flake) — no product impact.
- Sync plumbing, WebSocket transport, and the node↔hive protocol itself.
- Any new swarm capability. This restores and relocates existing surfaces; it does not add
  features.

## Findings this closes

- **F-2026-07-29-01** (high) — node Nodes page and swarm sections call removed `/api/nodes` and
  `/api/swarm` routes.
- **F-2026-07-29-02** (medium) — node board consumes `MergedProject` via the bridge endpoint;
  repoint to `Project`.

## Open questions for `/wai:spec`

1. Proxy route shape: restore the original `routes/nodes.rs` + `routes/swarm.rs` paths verbatim
   (frontend untouched), or introduce a namespaced prefix? Verbatim keeps the frontend diff at
   zero for part 1.
2. Does the API-key surface belong on the node at all, given `hive-node-api-key-ui` shipped it on
   the hive? Restoring `/api/nodes/api-keys` recreates the pre-fork duplication.
3. `MergedProject` retirement: delete the `#[ts(export)]` struct and `/api/merged-projects`
   outright, or keep the endpoint local-only (restored in `a85f7d63`) and only stop the frontend
   calling it? Deleting is cleaner; keeping is lower-risk if anything else consumes it.
4. Exact fate of the four remote stream hooks under C4 — `useNodeLogStream`, `useDiffStream`,
   `useRemoteConnectionStatus`, `useAvailableNodes`: proxy them like the rest, or make them
   no-op cleanly when hive-absent?
5. `verify_cmd` above assumes a hive-connected node with `VK_ORG_ID` exported. Confirm or replace
   with a check that works on the standard dev deploy.
