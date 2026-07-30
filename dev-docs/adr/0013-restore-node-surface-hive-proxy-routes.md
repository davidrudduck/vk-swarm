# ADR-0013 — Restore node-surface hive proxy routes (and keep the API-key surface deleted)

- **Status:** accepted
- **Date:** 2026-07-30
- **Workstream:** vk-swarm-node-ui-localize
- **Partially reverses:** `vk-swarm-node-foundations` removal of the node-surface API proxies
  (commit `35b378a5`, which deleted `crates/server/src/routes/{nodes,swarm_projects,swarm_labels,swarm_templates}.rs`)

## Context

node-foundations deleted the node's HTTP route layer for the nodes/swarm surface on the principle
that the node should be local-only. The **frontend was never updated to match**. The result on
`main` today is not a clean local-only node — it is a node whose swarm UI is still routed and
still shipping, and whose every data call 404s:

- `frontend/src/App.tsx:153` routes `/nodes` → `pages/Nodes.tsx` → `nodesApi` → `/api/nodes`
- `frontend/src/App.tsx:189` routes `/settings/swarm` → `SwarmSettings.tsx:163` →
  `SwarmProjectsSection`, `NodeProjectsSection`, `SwarmLabelsSection`, `SwarmTemplatesSection`,
  `NodeTemplatesSection` → `/api/swarm/*`, `/api/nodes/*`
- `crates/server/src/routes/mod.rs:44-71` registers no `nodes` or `swarm` module

The org picker at the top of `/settings/swarm` works, because `/api/organizations` survived as a
hive proxy (`routes/organizations.rs`, `deployment.remote_client()`). That makes the breakage read
as "no data" rather than "broken page", which is why it survived to `main`.

Two facts constrain the fix:

1. **`RemoteClient` was never gutted.** `crates/services/src/services/remote_client.rs` still
   carries `list_nodes`, `get_node`, `delete_node`, `list_node_projects`,
   `list_linked_node_projects`, `get_node_statuses`, and the complete
   `*_swarm_project` / `*_swarm_label` / `*_swarm_template` sets including merges and
   `promote_label_to_swarm`. Only the HTTP layer went.
2. **The browser cannot hold hive credentials.** `VK_NODE_API_KEY` is a server-side secret. A
   browser→hive-direct design would need browser-side auth plus CORS on the hive.

## Decision

**Restore the node-surface routes as thin `RemoteClient` pass-through proxies**, mirroring the
pattern already live in `routes/organizations.rs`. Restore the four deleted route modules
verbatim from `35b378a5^` (`git show 35b378a5^:crates/server/src/routes/<f>.rs`) and re-register
them in `routes/mod.rs`, so the frontend diff for this half is zero.

Proxy-through-node beats browser→hive-direct because it keeps `VK_NODE_API_KEY` server-side, needs
no CORS changes on the hive, and reuses an auth boundary that is already shipped and proven by
`organizations.rs`.

**What stays deleted:** the node's API-key management surface. `hive-node-api-key-ui` shipped key
management on the hive, and duplicating it on the node recreates the pre-fork duplication with a
second admin-gated path to the same privileged operation. Concretely, this workstream
**deletes** `frontend/src/components/org/NodeApiKeySection.tsx` and its mount at
`OrganizationSettings.tsx:380`, and does **not** restore `/nodes/api-keys`,
`/nodes/api-keys/{key_id}`, or any unblock route. Operators mint and revoke keys in the hive UI.

This does not revert node-foundations' principle — the node's **own** data stays local. It scopes
the principle correctly: *display and execution of the node's own work is local; management of
swarm-wide objects is a hive concern the node proxies to.*

## Consequences

- The node server regains four route modules. They contain no business logic — extract, call
  `RemoteClient`, wrap in `ApiResponse`.
- Hive-absent becomes a first-class UI state, not an error. `deployment.remote_client()` returns
  `RemoteClientNotConfigured`; every restored surface must render an explicit "not connected to a
  hive" state (spec SC4).
- `frontend/src/lib/api/nodes.ts` keeps `unblockApiKey` and the `merge-to` call with no server
  route behind them; both are removed with the API-key surface. The hive retains
  `/v1/nodes/api-keys/{key_id}/unblock` (`crates/remote/src/routes/nodes.rs:58`) and
  `/v1/nodes/{source_id}/merge-to/{target_id}` (`:68`) for its own UI.
- Pairing a brand-new node now requires the hive UI to mint the key. Accepted: that is already
  the documented onboarding path, and it is where `hive-node-api-key-ui` put it.
- `frontend/src/components/swarm/` and `remote-frontend/src/components/swarm/` remain two
  near-identical trees. Unification is explicitly out of scope; the hive tree is authoritative
  for behaviour, and the node tree is the proxy consumer.

## Alternatives rejected

- **Delete the node's swarm UI entirely.** Cleanest local-only story, but removes working
  operator screens for managing swarm projects, labels, templates, and node-project links from
  the node the operator is already looking at.
- **Browser→hive direct.** Requires solving browser-held hive credentials and CORS; rejected on
  C3 (`VK_NODE_API_KEY` is server-side).
- **Leave the pages gated behind a "manage this in the hive" empty state.** Leaves dead code and
  duplicate UI permanently, and still ships a screen that does nothing.
