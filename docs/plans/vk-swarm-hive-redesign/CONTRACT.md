# Cross-phase shared-interface contract — vk-swarm-hive-redesign (P2–P7)

The single source of truth for interfaces that SPAN phases. Authored up front so dependent phases do
not disagree on names/shapes (the parallel-authoring trap). Every Phase 2–7 task MUST conform; a task
that needs to diverge STOPs and the divergence is recorded here first.

## A. WS message variants (hand-duplicated in BOTH crates — Trap 3)

Defined identically in `crates/services/src/services/hive_client.rs` (node) AND
`crates/remote/src/nodes/ws/message.rs` (hive). Each new variant forces an arm in hive
`handle_node_message` (`session.rs:512`, EXHAUSTIVE) and an EXPLICIT arm before the node `_ =>`
wildcard (`hive_client.rs:1062`). **One protocol task per phase owns its variants; those tasks
`conflicts_with` each other on the two enum files** (sequenced by depends_on, authored against this
list so they never contradict).

| Phase | Direction | Variant | Payload |
|------|-----------|---------|---------|
| P1 (done) | node→hive | `NodeMessage::OpBatch` | `{ ops: Vec<OutboxOp> }` |
| P1 (done) | hive→node | `HiveMessage::OpAck` | `{ applied_through_seq: i64 }` |
| P2 | node→hive | `NodeMessage::LeaseHeartbeat` | `{ assignment_ids: Vec<Uuid> }` |
| P2 | hive→node | `HiveMessage::LeaseGrant` | `{ assignment_id: Uuid, fencing_token: i64, lease_expires_at: DateTime<Utc> }` |
| P2 | hive→node | `HiveMessage::LeaseRevoked` | `{ assignment_id: Uuid, reason: String }` |
| P5 | node→hive | `NodeMessage::Digest` | `{ entries: Vec<DigestEntry> }` (DigestEntry `{ entity_type: String, entity_id: Uuid, version: i64 }`) |
| P5 | hive→node | `HiveMessage::DigestResult` | `{ resend_from_seq: Option<i64>, pull_entities: Vec<Uuid> }` |

P3 (status) and P4 (inbound collapse) add **no** new WS variant — status rides the P1 op-log
(`task.upsert` carries status, gated by the P3 matrix); P4 changes inbound apply semantics, not the
wire.

## B. Schema additions

| Phase | DB | Change |
|------|----|--------|
| P2 | Postgres (hive) | `ALTER TABLE node_task_assignments ADD COLUMN lease_expires_at TIMESTAMPTZ, ADD COLUMN fencing_token BIGINT NOT NULL DEFAULT 0`; a monotonic token source — `CREATE SEQUENCE node_fencing_token_seq` (per-hive monotonic; the grant does `nextval`). |
| P7 | Postgres (hive) | one-time cutover migration(s) per ADR-0011 (migrate / regenerate / discard); preserves the `shared_task_id ↔ source_task_id` id bridge; remaps status enum values. |

No node-SQLite schema change beyond P1's `node_outbox` (its `fencing_token INTEGER` column already
exists for P2 to populate).

## C. Fencing semantics (P2, consumed by P3/P6)

- The node stamps `OutboxOp.fencing_token` (P1 column) **only for ops against a hive-assigned task**;
  node-owned work leaves it `NULL` (ADR-0009 scope). The hive `handle_op_batch` (106) gains a fencing
  check: for an op whose task has an active assignment, **reject (skip, do NOT apply, do NOT advance
  high-water past a rejected op — return an error/`LeaseRevoked`) if `op.fencing_token < assignment.fencing_token`**.
- Node self-fence: a renew-deadline watchdog halts the agent (reuse ADR-0001 process-group fence) when
  a lease cannot be renewed within its TTL.
- Lease-expiry sweep: a hive timer (analog of `crates/remote/src/services/stale_cleanup.rs`) reclaims
  assignments whose `lease_expires_at < now()`, bumping `fencing_token = nextval(...)`.

## D. Status state machine (P3, ADR-0010) — RECONCILED to the real enum (ratified 2026-06-30)

`task.status` ∈ `{Todo, InProgress, InReview, Done, Cancelled}` (both crates) — there is **no
`Assigned`/`Failed` variant**. SC4's `assigned`/`failed` are authority LABELS, not status values:
`assigned` = an active `node_task_assignments` row (hive, assignment layer); `failed` = an
`execution_status` outcome (node, execution layer). The **`task.status` single-author matrix**:
- **Hive-authored:** `todo→in-progress` (assign+start), `in-review→done`/`in-review→in-progress`
  (operator review), `*→cancelled`.
- **Node-reported** (only with a valid lease + current fencing token, CONTRACT §C):
  `in-progress→in-review`, `in-progress→done`.

One canonical wire value (node lowercase `inprogress`/`inreview` ↔ hive `in-progress`/`in-review` — the
mapping P1/106-F5 pins). See ADR-0010 for the full reconciliation rationale.

## E. Inbound single channel (P4, ADR-0007)

WS activity stream = single live inbound channel; bulk snapshot demoted to cold-start/gap-fill reconcile
only; the dead `ElectricTaskSyncService` task-shape path removed. One delete semantic = soft-unlink +
tombstone (clear `shared_task_id`, keep local `task_attempt`), identical on both legs. Node dirty-guard:
an inbound update never overwrites a field with an unacked outbound op. Handle `task.reassigned`
(`processor.rs:77` gap).

## F. No-fanout (P6, ADR-0007 data plane) — verify+guard

Investigation found the node-facing channel already carries only `ProjectSync`/`NodeRemoved` (NOT shared
task state). P6 = a test/guard asserting the invariant + a comment fence; NOT a large removal. Browser
fan-out (`electric_proxy`/`ActivityBroker`) is OUT of scope (hive-UI data source).
