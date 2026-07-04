---
workstream: vk-swarm-hive-redesign
doc_type: readme
status: shipped
title: "Phase 2b — rebuild hive as hub-and-spoke central management (after node)"
depends_on: [vk-swarm-node-foundations]
adrs: [0007, 0008, 0009, 0010, 0011]
staging_pointers:
  - dev-docs/workstreams/vk-swarm-hive-redesign/plans/vk-swarm-hive-redesign
  - dev-docs/workstreams/vk-swarm-hive-redesign/spec/2026-06-26-vk-swarm-hive-redesign.md
---

# vk-swarm-hive-redesign

Phase 2b (child of `vk-swarm-refactor`, **depends on** `vk-swarm-node-foundations`). Replace
bidirectional multi-master sync with a hub-and-spoke central management hive: ordered ack'd node→hive
outbox, lease/atomic-checkout assignment, explicit status state machine, anti-entropy self-heal,
collapsed inbound channels, central management UI, and a hive-only-state migration inventory. Analysis
basis: `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md` §2.
