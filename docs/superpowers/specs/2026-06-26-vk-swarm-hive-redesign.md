---
doc_type: spec
status: draft
workstream: vk-swarm-hive-redesign
change_kind: behaviour
---

# vk-swarm-hive-redesign — Rebuild the hive as a hub-and-spoke central management layer (Phase 2b)

> **Child of** [`vk-swarm-refactor`](./2026-06-25-vk-swarm-refactor.md). **Depends on**
> [`vk-swarm-node-foundations`](./2026-06-26-vk-swarm-node-foundations.md) — starts only after the
> node works 100% standalone.
>
> **Analysis basis (do not duplicate):** the sync root-cause verdict and evidence live in
> [`docs/specs/2026-06-25-vk-swarm-phase1-analysis.md`](../../specs/2026-06-25-vk-swarm-phase1-analysis.md)
> §2 (and §2.7, redirected below). This PRD captures *intent*; `/wai:spec` designs the protocol.

## Intent (what / why)

Replace the current ad-hoc **bidirectional multi-master** sync (field-level last-write-wins, two
divergent inbound channels, node↔hive↔node fan-out) with a **hub-and-spoke central management layer**.
Phase 1 found the sync instability is **architectural, not migration debt** (§2): a child
`task_attempt` can sync before its parent `task` is linked, dirty flags clear before any Hive ack
(silent write loss), and two inbound channels apply the same change with divergent semantics. The
maintainers' live-data `reset_*` repair migrations are the evidence.

The new model partitions authority by data type and direction:
- The **hive is the central management plane**: it owns the global board and cross-node management, and
  its web UI reads Postgres directly — no fan-out of shared task state to nodes.
- **Nodes report up** through a single durable, ordered, acknowledged outbox; they **do not** render or
  manage other nodes' state (enforced by `vk-swarm-node-foundations` SC5).
- The hive **assigns work down** with lease/atomic-checkout semantics.

This is the standard reliable fleet/agent control-plane shape — the same pattern as the **paperclip**
reference (`/data/Code/reference/agents/paperclip`: REST heartbeat, atomic task checkout, budgets,
approval gates). Topology change *eliminates the concurrent-write conflict class*; the ordering,
status-ownership, and assignment-safety problems below are designed in explicitly (topology alone does
not solve them).

## Users / who is affected

- **The operator** — gains a single central UI to manage all nodes/projects/tasks, instead of
  per-node UIs and node↔node coordination.
- **Multi-node swarm users** — relieved of the §2 corruption class (no more `reset_*` repairs).
- **Downstream phases (P3, P6)** — the AI breakdown harness and management agent get a single
  authoritative management plane and event source to drive.

## Success criteria

- SC1: The hive is the **central management plane** — its web UI manages all connected nodes,
  projects, tasks, attempts, and executions, reading from Postgres directly with **no node↔node or
  node↔hive↔node fan-out**.
- SC2: Node→hive sync uses a **single ordered, acknowledged per-node outbox/op-log** with idempotency
  keys; **cross-entity ordering** (parent task before child attempt) is guaranteed by construction and
  there is **zero silent write loss** (dirty state clears only after hive durable commit). *(Clauses:
  SC2a ordered single channel; SC2b parent-before-child; SC2c ack'd no-loss.)*
- SC3: Hive→node **assignment uses lease / atomic-checkout** (heartbeat + lease expiry + idempotent
  claim); a network partition **cannot cause double execution** of the same task.
- SC4: `task.status` transitions follow an **explicit state machine** naming which transitions are
  **hive-authored** (`assigned`, `in-progress`) vs **node-reported** (`done`, `failed`); no field-level
  conflict on status.
- SC5: Node↔hive divergence **self-heals** via an anti-entropy reconciliation sweep — **no manual
  `reset_*` migration** is ever required.
- SC6: A **migration inventory** of all state that lives *only* in the hive today (cross-node
  assignments, board organization, manual hive-UI edits) is complete, and that state is migrated or
  explicitly handled on cutover to the rebuilt hive.
- SC7: The two inbound channels (Electric SQL shape poll + WS activity stream) are **collapsed to one**,
  with **one delete semantic and one conflict policy** applied uniformly.

## Constraints

- **Keep:** Postgres as the hive store, WebSocket transport, local SQLite as node-of-record. Rebuild
  the *reconciliation contract and management surface*, not the storage engines.
- **Start only after `vk-swarm-node-foundations` is complete** — the rebuilt hive re-ingests
  node-authoritative state via the new outbox; nodes must be the reliable source first.
- **Resolve the redirected §2.7 check before the cutover design hardens:** the open question is no
  longer "targeted vs rebuild" (rebuild is chosen) but the **migration inventory** (SC6) plus the
  broadcast-fan-out and SQLite↔Postgres schema-drift checks.
- **Nodes manage local-only;** the central management surface lives here, not on nodes
  (`vk-swarm-node-foundations` SC5 is the enforcing boundary).
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **Node-local durability and crash-resumability** — owned by `vk-swarm-node-foundations`.
- **Hive HA / replication** — a named *future* requirement (every node retains its always-on local UI
  as the management fallback during a hive outage); not in this workstream.
- **Schema/migration consolidation** beyond what the rebuild requires — hygiene, deferred.
- **P3+ AI breakdown / event bus / management agent** — consumers of this plane, designed later.
