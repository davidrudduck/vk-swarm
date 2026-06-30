---
doc_type: spec
status: active
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

## User stories

- **US1:** As the operator of a multi-node swarm, when I manage nodes/projects/tasks, I expect a single
  central hive UI reading Postgres directly, with no node↔node or node↔hive↔node fan-out.
- **US2:** As a multi-node swarm user, when a node writes locally over a flaky link, I expect node→hive
  sync on one ordered, acknowledged channel with correct parent-before-child ordering and zero silent
  write loss.
- **US3:** As the operator, when the network partitions mid-run, I expect hive→node assignment to use
  lease / atomic-checkout so the same task is never double-executed.
- **US4:** As a swarm user, when a task moves through its lifecycle, I expect status transitions to
  follow an explicit state machine with no field-level conflict.
- **US5:** As the operator, when a node and the hive diverge, I expect self-healing reconciliation so I
  never have to run a manual `reset_*` repair migration.
- **US6:** As the operator, when we cut over to the rebuilt hive, I expect all hive-only state to be
  inventoried and either migrated or explicitly handled — nothing silently lost.
- **US7:** As a swarm user, when the hive delivers a change to a node, I expect one inbound channel with
  one delete semantic and one conflict policy applied uniformly.

## Success criteria

- SC1: The hive is the **central management plane** — its web UI manages all connected nodes,
  projects, tasks, attempts, and executions, reading from Postgres directly with **no node↔node or
  node↔hive↔node fan-out**. → US1
- SC2: Node→hive sync uses a **single ordered, acknowledged per-node outbox/op-log** with idempotency
  keys; **cross-entity ordering** (parent task before child attempt) is guaranteed by construction and
  there is **zero silent write loss** (dirty state clears only after hive durable commit). *(Clauses:
  SC2a ordered single channel; SC2b parent-before-child; SC2c ack'd no-loss.)* → US2
- SC3: Hive→node **assignment uses lease / atomic-checkout** (heartbeat + lease expiry + idempotent
  claim); a network partition **cannot cause double execution** of the same task. → US3
- SC4: `task.status` transitions follow an **explicit state machine** naming which transitions are
  **hive-authored** (`assigned`, `in-progress`) vs **node-reported** (`done`, `failed`); no field-level
  conflict on status. → US4
- SC5: Node↔hive divergence **self-heals** via an anti-entropy reconciliation sweep — **no manual
  `reset_*` migration** is ever required. → US5
- SC6: A **migration inventory** of all state that lives *only* in the hive today (cross-node
  assignments, board organization, manual hive-UI edits) is complete, and that state is migrated or
  explicitly handled on cutover to the rebuilt hive. → US6
- SC7: The two live inbound channels (REST bulk-snapshot reconcile + WS activity stream) are
  **collapsed to one**, with **one delete semantic and one conflict policy** applied uniformly. (The
  Electric SQL `shared_tasks` shape poll named in analysis §2.3 is **dead in code** — no Rust caller;
  it is removed, not collapsed.) → US7

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

## Approach

Rebuild the **reconciliation contract and management surface** as a hub-and-spoke control plane while
keeping the storage engines (Postgres hive store, SQLite node-of-record, WS transport — per
Constraints). The rebuild itself is settled at program level (analysis §2.6); this design fixes the four
defect classes Phase 1 proved by collapsing channels and adding the guarantees topology alone does not
give. The three §2.7 open items were **resolved in code** (read-only investigation, cited below), as the
Constraints require before the cutover design hardens:

- **Cross-entity ordering is already declaratively enforced** (`hive_sync.rs:340-349` skip-when-unlinked
  + JOIN guards `execution_process/sync.rs:22-23`, `log_entry/sync.rs:26-27`) — *guardable*, so the
  op-log preserves it by construction rather than re-deriving it. The real defect is **ack timing**
  (flags clear on mpsc-enqueue, not durable ack → silent write loss).
- **The two inbound channels can double-deliver and diverge on delete** (one hive tx writes
  `shared_tasks` + an activity row, `remote/src/db/tasks.rs:892-893`; bulk-snapshot **hard-deletes**
  vs WS activity **soft-unlinks**), and `task.reassigned` is dropped by the activity processor
  (`processor.rs:77`). The live second channel is the **REST bulk-snapshot reconcile**
  (`node_runner.rs:986`); the Electric `shared_tasks` shape path is **dead** (no Rust caller; frontend
  subscribes only to `nodes`/`projects` shapes) — SC7's parenthetical was corrected to reflect this.
- **`tasks` column drift is moderate-high**, with a load-bearing **status enum value mismatch**
  (`inprogress`/`inreview` vs hyphenated `in-progress`/`in-review`) and **two distinct id spaces**
  bridged by `shared_task_id` ↔ `source_task_id`.

The **node-side outbox writer and the heartbeat/fencing client are new work introduced here** —
`vk-swarm-node-foundations` deliberately left the node↔hive sync *contract* out of scope (its SC5 only
stripped the node UI; sync plumbing was left intact). This workstream owns both ends of the contract.

## Design / architecture

Each mechanism maps 1:1 to a success criterion and a backing ADR.

**1. Hive as central management plane, zero fan-out (SC1).** The hive web UI manages all
nodes/projects/tasks/attempts/executions reading Postgres directly. Hive→node traffic is **only
assignments and acks directed at that node** — never a replica of the global board pushed to every
node. The node→node and node↔hive↔node fan-out paths (`remote/src/activity/broker.rs` broadcast) are
removed for shared task state. This dissolves the §2 concurrent-write conflict class at the topology
level (inherited program decision; analysis §2.6).

**2. Single ordered, acknowledged outbox / op-log (SC2 · [ADR-0008](../../../dev-docs/adr/0008-node-hive-ordered-ack-outbox.md)).**
Replace the five node→hive push paths (`hive_sync.rs:303/368/434/562`, `share/publisher.rs:109`) with
one per-node append-only `outbox(seq, op_type, entity_type, entity_id, payload, idempotency_key,
fencing_token, acked_at)` in node SQLite. Ops stream in `seq` order over the WS; the hive applies
idempotently (`idempotency_key` UNIQUE + `ON CONFLICT DO NOTHING`), **parks a child op until its parent
applies** (SC2b), and returns a **durable ack** carrying the applied high-water `seq`. The node clears
`acked_at`/advances its cursor **only on durable ack** (SC2c) — closing the silent-loss window. Per-op
monotonic version replaces the hardcoded `version: 1` (`hive_sync.rs:289`).

**3. Lease / atomic-checkout assignment, partition-safe via fencing (SC3 · [ADR-0009](../../../dev-docs/adr/0009-lease-checkout-fencing.md)).**
The hive assigns by atomic CAS (`UPDATE node_task_assignments … WHERE <available> RETURNING`) — no two
nodes both claim. Each grant carries `lease_expires_at` + a monotonic **fencing token**; the node
renews via heartbeat. On missed renewal the hive reclaims with a **higher** token. **Fencing is scoped
to hive-assigned tasks:** the node stamps every outbox op **against a hive-assigned task** with that
task's current lease token, and the hive **rejects stale-token commits** on assigned tasks →
**at-most-once commit effect**. Node-owned work (locally-created tasks and their attempts/execs/logs —
the majority of outbox traffic) carries **no lease token** and is committed under the node's ownership
identity, not a lease. The node **self-fences** (halts the agent via ADR-0001's process-group kill) when
it cannot renew → **bounded execution overlap**. Together: a partition cannot cause double-execution
*effect*. This **discharges node-foundations D7** (foreign-row disambiguation).

**4. Explicit `task.status` state machine (SC4 · [ADR-0010](../../../dev-docs/adr/0010-task-status-state-machine.md)).**
A guarded transition matrix with one authoritative author per transition: **hive-authored**
`todo→assigned`, `assigned→in-progress`, `*→cancelled`; **node-reported** `in-progress→done`,
`in-progress→failed` (accepted only with a valid lease + fencing token). No field-level status merge.
One canonical wire value resolves the `inprogress`/`in-progress` drift.

**5. Anti-entropy reconciliation (SC5 · [ADR-0008](../../../dev-docs/adr/0008-node-hive-ordered-ack-outbox.md)).**
Riding the op-log: on reconnect and periodically the node re-streams from its unacked cursor, and a
**digest exchange** (per-entity version/hash + outbox high-water) detects silent divergence the cursor
misses; the hive replies with the ops/pulls to heal. The repurposed bulk-snapshot reconcile
([ADR-0007](../../../dev-docs/adr/0007-single-inbound-channel-one-delete-one-conflict.md)) is its
gap-fill leg. This *replaces* the manual `reset_*` repair migrations.

**6. Hive-only-state cutover inventory (SC6 · [ADR-0011](../../../dev-docs/adr/0011-hive-only-state-cutover.md)).**
The redirected §2.7. Full read-only inventory classifies every hive-only table as **must-migrate**
(`node_api_keys`, `nodes`, active `node_task_assignments`, `swarm_projects`/`swarm_project_nodes`,
`swarm_templates`, `shared_tasks` incl. attribution + the id bridge, `labels`/`shared_task_labels`,
identity/tenancy), **regenerable** (node-mirror caches/logs/sync bookkeeping — rebuilt by node
re-ingest), or **discardable** (activity history, sessions, completed assignments). **No board-ordering
table exists** (order derived from `status`+`activity_at`) — no layout to migrate. Cutover: run the
must-migrate migration preserving the id bridge, then nodes reconnect and re-ingest regenerable state.

**7. Single inbound channel, one delete, one conflict (SC7 · [ADR-0007](../../../dev-docs/adr/0007-single-inbound-channel-one-delete-one-conflict.md)).**
WS activity stream becomes the single live inbound channel; the bulk snapshot is demoted to
cold-start/gap-fill reconcile; the dead Electric task-shape path is removed. One delete semantic
(hive soft-delete → node **soft-unlink + tombstone**, preserving local attempts) applied identically on
both legs. One conflict policy: hive authoritative + node **dirty-guard** (an inbound update never
overwrites a field with an unacked outbound op) — replacing the `remote_version`-only gate that silently
clobbers local edits (`task/sync.rs:300`, `task/queries.rs:305-307`). The single channel handles
`task.reassigned`.

## Decisions

- **D1 — Collapse hive→node delivery to one live channel (WS activity), reconcile-only bulk snapshot,
  delete the dead Electric task path; one delete semantic + one conflict policy.** *(Irreversible:
  wire-format + code delete + delete-semantic change.)* → [ADR-0007](../../../dev-docs/adr/0007-single-inbound-channel-one-delete-one-conflict.md)
- **D2 — Replace the five node→hive push paths with one ordered, ack'd per-node op-log/outbox**;
  cursor advances only on durable ack; parent-before-child by construction; anti-entropy digest rides
  it (SC5). *(Irreversible: wire format + deletes legacy push code.)* → [ADR-0008](../../../dev-docs/adr/0008-node-hive-ordered-ack-outbox.md)
- **D3 — Assignment = atomic checkout + lease, made partition-safe by fencing tokens + node
  self-fencing** (at-most-once effect, bounded overlap); **discharges node-foundations D7**.
  *(Irreversible: assignment contract + fencing-token wire field.)* → [ADR-0009](../../../dev-docs/adr/0009-lease-checkout-fencing.md)
- **D4 — `task.status` is an explicit single-author transition matrix**; one canonical status wire
  value (resolve `inprogress`/`in-progress` drift). *(Irreversible: contract + enum value
  canonicalization.)* → [ADR-0010](../../../dev-docs/adr/0010-task-status-state-machine.md)
- **D5 — Hive-only state cutover** → [ADR-0011](../../../dev-docs/adr/0011-hive-only-state-cutover.md) *(Irreversible: data migration / drop)*. Classified migrate /
  regenerate / discard with a pre-cutover backup; the node↔hive id bridge is preserved; no
  board-ordering table to migrate.
- **D6 — The node-side outbox writer + heartbeat/fencing client are new work owned here** (not by
  `vk-swarm-node-foundations`, which left sync plumbing intact). *(Reversible scope note.)*
- **D7 — SC7's parenthetical was corrected** from "Electric SQL shape poll" to "REST bulk-snapshot
  reconcile" — a factual fix (the Electric task path is dead in code), not an intent change; the
  collapse-two-to-one criterion is unchanged. *(Reversible doc correction.)*

## Test strategy

- **TS1: Ordered ack'd outbox, no silent loss (SC2).** With the `qa_mock` executor and a fault-injected
  WS, drop the connection between op-enqueue and hive ack; assert the op is **retried** (not cleared)
  and applies exactly once (idempotency key); assert a child op is **parked** until its parent applies.
  Use `db::test_utils::create_test_pool()` for the node side.
- **TS2: Partition cannot double-execute (SC3).** Lease a task to node A; simulate A partitioned but
  alive; expire + reassign to B (higher fencing token); assert A's late commit is **rejected by
  stale-token** and A **self-fences** (agent halted) within TTL — at-most-once effect, bounded overlap.
- **TS3: status state machine (SC4).** Table-driven over the transition matrix: each legal transition is
  accepted from its sole author and **rejected** from the other party / without a valid lease+token;
  assert the `inprogress`↔`in-progress` value maps to one canonical wire form.
- **TS4: anti-entropy self-heal (SC5).** Seed a divergence (node has an entity the hive lacks and vice
  versa); run the digest-exchange sweep; assert convergence with **no `reset_*`-style manual step**.
- **TS5: single inbound channel semantics (SC7).** Assert a hive soft-delete yields **one** node
  outcome (soft-unlink + tombstone, local attempt retained) regardless of leg; assert a concurrent
  local edit is **not** clobbered by an inbound update (dirty-guard); assert `task.reassigned` is
  applied on the single channel.
- **TS6: cutover inventory (SC6).** A migration test asserts every **must-migrate** table round-trips
  into the rebuilt schema with the id bridge intact and status values remapped; **regenerable** tables
  repopulate from a simulated node re-ingest; **discardable** tables are absent — nothing in the
  inventory is silently lost.
- **TS7: no fan-out (SC1).** Assert a task owned by node X is **not** pushed to node Y; node Y's inbound
  stream contains only its own assignments/acks (topology check).
