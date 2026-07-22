---
workstream: vk-swarm-node-foundations
doc_type: readme
status: shipped
title: "Phase 2a — node correct, durable, crash-resumable (ships first)"
adrs: [0001, 0002, 0003]
staging_pointers:
  - dev-docs/workstreams/vk-swarm-node-foundations/plans/vk-swarm-node-foundations
  - dev-docs/workstreams/vk-swarm-node-foundations/spec/2026-06-26-vk-swarm-node-foundations.md
---

# vk-swarm-node-foundations

Phase 2a (child of `vk-swarm-refactor`, ships **first**). Make a single node fully correct, durable,
and crash-resumable standalone: re-spawn-with-`--resume` recovery, durable workstream-state object,
persisted message queue, local-durability audit, stripped-back local-only UI + read-only hive-sync
view, and 3 forward-ported stability fixes. The node↔hive sync contract is **out of scope** (see
`vk-swarm-hive-redesign`). Analysis basis: `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md`.
