---
workstream: vk-swarm-node-ui-localize
doc_type: readme
status: draft
title: "Localize the node frontend — repoint dual-purpose remote-display hooks to local-only"
staging_pointers: []
depends_on: [vk-swarm-node-foundations]
adrs: []
---

# vk-swarm-node-ui-localize

**Carved out of `vk-swarm-node-foundations` Phase 4 by user decision (2026-06-26)** when decompose found
the remaining remote-display removal is an entangled multi-component frontend refactor, not the clean
deletes ADR-0002 assumed. node-foundations delivers the local-only core (backend visibility
discriminator, removal of the request-time remote merge + node-surface API proxies, deletion of the
self-contained Nodes-management feature, read-only hive-sync view). This workstream finishes the job in
the React layer.

No spec yet — this is a tracker stub for a future `/wai:prd-new` + `/wai:spec` + `/wai:precheck` +
`/wai:decompose`, sequenced AFTER node-foundations ships.

## Scope (the entangled remainder)

Repoint / remove the dual-purpose remote-aware frontend so the node's local views render local-only
state. The entanglement map (verified during node-foundations decompose):

- **`useMergedProjects` → `useProjects`.** `ProjectList` / `ProjectSwitcher` are *typed on*
  `MergedProject` (the `#[ts(export)] MergedProject` struct, deliberately kept by node-foundations task
  403 to stay codegen-neutral). Repointing them to `Project` is a non-trivial typed refactor.
- **Remote card badges** on local task/project cards (remote-state indicators).
- **Remote stream/diff hooks wired into live local components:**
  - `useNodeLogStream` → `ProcessLogsViewer`
  - `useDiffStream` → `DiffsPanel` / `useDiffSummary`
  - `useRemoteConnectionStatus` → `AttemptHeaderActions`
  - `useAvailableNodes` → `CreateAttemptDialog`
- **`SwarmSettings.tsx`** (imports entangled `@/components/swarm` sections) — node-foundations task 405
  sidestepped it by adding a self-contained `HiveSyncStatusCard` to `SystemSettings.tsx`; this
  workstream decides the fate of the remaining swarm settings UI.

## Keep (do NOT remove — live non-Nodes consumers)

`useNode`, `nodesApi` (`lib/api/nodes.ts`), `components/org/NodeApiKeySection.tsx`,
`components/swarm/NodeProjectsSection.tsx` — these have live consumers outside the deleted Nodes
feature; node-foundations explicitly kept them.

## Relationship to the program

Child of `vk-swarm-refactor` (the umbrella). Depends on `vk-swarm-node-foundations`. Independent of
`vk-swarm-hive-redesign`. Pure frontend; touches no sync plumbing or backend contracts.
