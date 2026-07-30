---
workstream: vk-swarm-node-ui-localize
doc_type: readme
status: draft
title: "Localize the node frontend — proxy swarm management to the hive, make the board local-only"
staging_pointers:
  - docs/superpowers/specs/2026-07-30-vk-swarm-node-ui-localize.md
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

## Entanglement map is STALE — re-verify before spec (2026-07-29)

`vk-swarm-node-foundations` is now `shipped`, so this workstream is unblocked. But the "Keep (do
NOT remove)" section above was verified during node-foundations *decompose* and no longer matches
merged code. Do not trust it as written — re-derive it during `/wai:prd-new` + `/wai:spec`.

Two contradictions found against `main`:

1. **The Nodes feature was NOT deleted.** `frontend/src/App.tsx:153` still routes
   `/nodes` → `frontend/src/pages/Nodes.tsx` → `nodesApi`.
2. **The "live consumers" are calling dead endpoints.** `nodesApi` (`lib/api/nodes.ts`) and
   `swarmProjects`/`swarmLabels` target `/api/nodes` and `/api/swarm`, and
   `crates/server/src/routes/mod.rs:44-71` registers neither module — every request 404s.
   `NodeProjectsSection` is reachable in production via `SwarmSettings.tsx:163`
   (routed at `App.tsx:189` as `/settings/swarm`, plus `MobileSettingsAccordion.tsx:60`).

So the open question the spec must answer is **repoint vs delete** — the "Keep" rationale assumed
these had healthy local consumers, and they do not. Tracked as F-2026-07-29-01 (high) and
F-2026-07-29-02 in `dev-docs/BACKLOG.md`.

## Intent captured (2026-07-30)

`/wai:prd-new` ran; intent doc at
`docs/superpowers/specs/2026-07-30-vk-swarm-node-ui-localize.md`. It **supersedes the "Scope" and
"Keep" sections above** — treat those as historical.

User decisions taken at capture:
1. **Repoint, don't delete.** The node keeps its swarm/nodes UI; the node server regains thin
   `RemoteClient` proxy routes mirroring `routes/organizations.rs`. Tractable because
   node-foundations deleted only the HTTP layer — `RemoteClient` retains every needed method.
2. **Full entanglement list in scope** — remote card badges and all four remote stream/diff hooks,
   not just the two findings.
3. **Board is local-only** — `useMergedProjects` → `useProjects`, `MergedProject` retired.

Blocking before `/wai:decompose`: an **ADR for restoring the node-surface proxies** (constraint C2)
— it partially reverses ADR-0002 / node-foundations.
