---
topic: vk-swarm-hive-redesign
doc_type: plan
status: draft
spec: docs/superpowers/specs/2026-06-26-vk-swarm-hive-redesign.md
---

# Plan — vk-swarm-hive-redesign

## Approach

Rebuild the node↔hive reconciliation contract as a hub-and-spoke control plane, keeping the storage
engines (Postgres hive store, SQLite node-of-record, WS transport). The work spans **both ends of a
network protocol** and is sequenced so the **foundation flows end-to-end before anything rides it**:
the ordered, acknowledged op-log (SC2) is the channel every later guarantee depends on, so it ships
first as a tracer bullet — one op type flowing node→hive with a durable ack — then the remaining op
types, then the mechanisms that ride it (fencing, status machine, anti-entropy). The inbound-channel
collapse (SC7) and the cutover migration (SC6) are comparatively independent and ship in their own
phases.

Every task is Rust (or a migration/CI/frontend task). **This repo is a Cargo workspace, so the WAI
gate's TypeScript type-check is skipped and its `scope_test` runner has no native `cargo test` path** —
every Rust task carries explicit `WAI_TYPECHECK_CMD`/`WAI_TEST_CMD` overrides (decisions-ledger Trap 1).
**SQLx is offline-mode** (committed `.sqlx` cache, `DATABASE_URL` unset): schema/query tasks execute
against a live migrated dev DB and never run `cargo sqlx prepare` in a gated task (Trap 2). Adding any
`NodeMessage`/`ServerMessage` WS variant (the op-log/ack/heartbeat/lease messages) hits the
**enum-exhaustiveness trap on BOTH ends** (Trap 3) — every match arm in the same commit. Anchors were
authored against current `main`; the adversarial breakdown tournament re-verifies each before any code.

**This decompose covers the protocol + data plane (SC2–SC7 and the no-fan-out, data-plane half of
SC1).** The *hive central-management web UI* half of SC1 is a carve candidate (see Scope note) decided
with the user before task authoring, mirroring the node-foundations → `vk-swarm-node-ui-localize` split.

Phase dependency spine: **P1 (op-log) → P2 (fencing) → P3 (status machine)**; **P1 → P5 (anti-entropy)**;
**P4 (inbound collapse)** is independent of the spine; **P7 (cutover migration)** depends on the
rebuilt schema (P1–P3); **P6 (no fan-out)** is data-plane and independent.

## Phases

1. **phase-1-oplog** — node→hive single ordered, acknowledged op-log/outbox: outbox table, op append on
   local writes, WS stream in seq order, hive idempotent apply + durable ack, cursor-advance-on-ack,
   parent-before-child parking. Replaces the five push paths (SC2). *Foundation.*
2. **phase-2-lease-fencing** — assignment via atomic checkout + lease (`lease_expires_at`), heartbeat
   renewal, monotonic fencing token on grants and on op-log ops, hive stale-token rejection, node
   self-fencing (reuses the ADR-0001 process fence). Discharges node-foundations D7 (SC3). Depends P1.
3. **phase-3-status-machine** — explicit `task.status` transition matrix, single-author enforcement
   (hive vs node, gated on lease+token), status enum value canonicalization (SC4). Depends P1, P2.
4. **phase-4-inbound-collapse** — WS activity stream as the single live inbound channel; bulk snapshot
   demoted to reconcile-only; remove the dead Electric task-shape path; one delete semantic
   (soft-unlink + tombstone); node dirty-guard conflict policy; handle `task.reassigned` (SC7).
5. **phase-5-anti-entropy** — digest exchange (per-entity version/hash + outbox high-water) over the
   op-log, gap heal replacing manual `reset_*` migrations (SC5). Depends P1 (+P4 reconcile leg).
6. **phase-6-no-fanout** — **verify + guard** that nodes receive only assignment/ack traffic, never
   pushed shared-task state (SC1 data-plane half). Investigation found this is *already* true today —
   the node-facing channel carries only `ProjectSync`/`NodeRemoved` — so phase-6 asserts and fences the
   invariant against regression; it is NOT a large fan-out removal. Browser-facing shared-task fan-out
   (electric_proxy/ActivityBroker) is the hive-UI data source and is OUT of scope. Depends P1, P2
   (prerequisite tasks 103 — the `HiveMessage` enum drift fence — plus 202 and 501, whose
   variant-adding tasks grow the enum 601's exhaustive `match` classifies).
7. **phase-7-cutover** — one-time hive-only-state migration (migrate / regenerate / discard per
   ADR-0011), preserving the node↔hive id bridge and remapping status enum values (SC6). Depends P1–P3
   (the rebuilt schema P1/P2 add — `node_op_log`, `node_task_assignments` lease columns — and the P3
   status canonicalization must all be present before the cutover guards can assert them).

## Scope note — SC1 central-management UI CARVED to `vk-swarm-hive-ui`

SC1 has two halves: (a) the **data-plane** guarantee "no node↔node / node↔hive↔node fan-out" (covered
here, phase-6), and (b) the **hive web UI manages all** nodes/projects/tasks/attempts/executions.
**User decision (2026-06-30): half (b) is carved into `vk-swarm-hive-ui`** (tracker seeded; rehost
rewire + net-new cross-node views — see that README and the decisions ledger). This plan covers SC2–SC7
+ SC1's data-plane half. SC1 stays in the frozen spec, claimed by phase-6; no spec edit was made.

## Task table

`dep:`/`conflicts:` mirror each task's frontmatter (wai-plan-lint enforces equality). `-` = none.
**Phase 1 is authored (this pass); Phases 2–7 are authored in subsequent `/wai:decompose` passes** as
each prior ships (user-approved phase-by-phase, decisions-ledger).

### Phase 1 — op-log foundation (SC2; authored as a safe additive tracer, op_type `task.upsert`)

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 101 | Add `node_outbox` table migration (SQLite) | dep: - | conflicts: none | SC2a |
| 102 | Add `node_op_log` table migration (Postgres/hive) | dep: - | conflicts: none | SC2c |
| 103 | Add `OpBatch`/`OutboxOp`/`OpAck` WS variants to both crates + exhaustive stub arms | dep: - | conflicts: 106 108 | SC2a |
| 104 | Add node `OutboxRepository` (enqueue/peek_unacked/mark_acked_through) | dep: 101 | conflicts: none | SC2a |
| 105 | Enqueue a `task.upsert` op on `Task::create`/`update` | dep: 104 | conflicts: none | SC2b |
| 106 | Hive `handle_op_batch` — idempotent apply + park/skip + durable `OpAck` | dep: 102 103 | conflicts: 103 | SC2c |
| 107 | Node streamer — drain `node_outbox` into `OpBatch` in `sync_once` | dep: 103 104 | conflicts: none | SC2a |
| 108 | Node `OpAck` — advance ack cursor on durable hive ack | dep: 103 104 | conflicts: 103 | SC2c |

> **Tracer honesty (recorded in the ledger):** Phase 1 proves the ordered-ack'd round-trip *mechanism*
> alongside the legacy paths. It does NOT yet fully discharge SC2 — (i) `105`'s enqueue is non-atomic
> with the task write, so true SC2c no-loss needs a transactional enqueue; (ii) only `task.upsert`
> flows; (iii) the five legacy push paths are NOT retired. Those are the next Phase-1 increment.

### Phase 2 — lease / atomic-checkout + fencing (SC3; ADR-0009; discharges node-foundations D7)

Atomic conditional checkout (`try_claim`) + real lease (`lease_expires_at`) + monotonic `fencing_token`
(per-hive `node_fencing_token_seq`). The node renews via `LeaseHeartbeat`; the hive replies `LeaseGrant`
(token+expiry). The node stamps `OutboxOp.fencing_token` for hive-assigned tasks **at op-stream time** from
its lease state (the `db`-crate enqueue cannot know leases — ledger), and the hive **rejects** any op whose
token is older than the assignment's current token (the at-most-once commit effect). A node that loses its
lease **self-fences** (reuses the ADR-0001 `stop_execution` kill via `AssignmentHandler::handle_cancellation`).
A background sweep (`stale_cleanup.rs` analog) reclaims expired leases with a bumped token. Variant shapes
are FIXED by CONTRACT §A; schema by §B; fencing semantics by §C.

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 201 | Add `lease_expires_at`+`fencing_token` cols + `node_fencing_token_seq` (Postgres) | dep: - | conflicts: none | SC3 |
| 202 | Add `LeaseHeartbeat`/`LeaseGrant`/`LeaseRevoked` WS variants to both crates + stub arms | dep: - | conflicts: 204 205 206 501 | SC3 |
| 203 | Hive `TaskAssignmentRepository::try_claim` (atomic CAS) + `renew_lease` | dep: 201 | conflicts: 209 | SC3 |
| 204 | Hive `handle_lease_heartbeat` — renew leases, reply `LeaseGrant` per assignment | dep: 202 203 | conflicts: 202 205 | SC3 |
| 205 | Hive fencing enforcement in `handle_op_batch` — reject stale-token, emit `LeaseRevoked` | dep: 106 202 203 | conflicts: 202 204 | SC3 |
| 206 | Node lease state — `HiveEvent` lease variants, token+expiry on `ActiveAssignment`, send `LeaseHeartbeat` | dep: 202 | conflicts: 202 207 208 | SC3 |
| 207 | Node — stamp `OutboxOp.fencing_token` from the lease at stream time (hive-assigned tasks only) | dep: 107 206 | conflicts: 206 208 | SC3 |
| 208 | Node self-fence watchdog — halt the agent on lease-revoke / renew-deadline miss | dep: 206 | conflicts: 206 207 | SC3 |
| 209 | Hive lease-expiry sweep — reclaim expired leases with a bumped token (timer, analog stale_cleanup) | dep: 201 203 | conflicts: 203 | SC3 |
| 210 | SC3 acceptance test — partition cannot double-execute (stale-token reject + self-fence) | dep: 203 205 208 209 | conflicts: none | SC3 |

> **Phase-2 honesty:** SC3's guarantee is "at-most-once commit *effect* (stale-token rejection, 205) +
> bounded-overlap execution (node self-fence, 208)" — NOT "we have leases" (ADR-0009). 210 claims TS2 and
> proves the hive reject leg end-to-end; the node self-fence leg is proven by 206/208's hermetic unit tests
> (a true cross-process WS round-trip is out of hermetic scope — recorded in the ledger). The lease state
> lives on `ActiveAssignment` (one structure serves 206/207/208) — a reconciled design decision (ledger).
> **Cross-phase shared files:** 205 EDITS `handle_op_batch` (authored by P1/106) — `depends_on: 106`. The
> WS enum files (`message.rs`, `hive_client.rs`), `session.rs`, and `node_runner.rs` are touched in BOTH
> P1 and P2 (and later phases); the orchestrator reconciles cross-phase conflicts on them.

### Phase 5 — anti-entropy reconciliation digest (SC5; rides P1's op-log, TS4 self-heal)

Riding the P1 op-log (ADR-0008): the node periodically (and on the first sync cycle after reconnect)
emits a per-entity **version digest** of its swarm-linked tasks; the hive compares it against
`shared_tasks`/`node_op_log` and replies a **DigestResult** that directs the heal —
`resend_from_seq` re-streams the node's op-log (node-has/hive-lacks, including acked-but-lost ops via the
new `peek_from_seq`) and `pull_entities` triggers the bulk-snapshot reconcile leg (hive-has/node-lacks).
This **replaces the manual `reset_*` repair migrations**; convergence is the protocol path only, no
out-of-band SQL. The `Digest`/`DigestResult` variant shapes are FIXED by CONTRACT §A.

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 501 | Add `Digest`/`DigestEntry`/`DigestResult` WS variants to both crates + exhaustive stub arms | dep: 202 | conflicts: 202 503 504 | SC5 |
| 502 | Node digest builder — emit `NodeMessage::Digest` of swarm-linked tasks each sync cycle | dep: 501 | conflicts: none | SC5 |
| 503 | Hive `handle_digest` — compare vs `shared_tasks`/`node_op_log`, reply `DigestResult` (TS4 self-heal) | dep: 501 | conflicts: 501 | SC5 |
| 504 | Node acts on `DigestResult` — re-stream from `resend_from_seq` + pull via reconcile leg | dep: 501 502 | conflicts: 501 | SC5 |

> **Cross-phase shared-file collision (CONTRACT §A, recorded in the ledger).** 501 hand-duplicates the
> `Digest`/`DigestResult` variants into `crates/services/src/services/hive_client.rs` AND
> `crates/remote/src/nodes/ws/message.rs` (+ exhaustive arms in `session.rs`) — the SAME two enum files
> P2's lease-protocol task edits (`LeaseHeartbeat`/`LeaseGrant`/`LeaseRevoked`). The two protocol tasks
> `conflicts_with` each other; P2 is **not yet authored as ids**, so 501's frontmatter carries only the
> intra-P5 conflicts. When P2 lands, its protocol task MUST add 501 to its `conflicts_with` (and 501 is
> updated symmetrically) and be `depends_on`-sequenced so the two never edit the enum tails at once.
> 503 also edits `crates/remote/src/nodes/ws/session.rs` and `crates/remote/src/db/tasks.rs`; 504 also
> edits `crates/db/src/models/node_outbox.rs` and `crates/services/src/services/node_runner.rs`.

### Phase 6 — no fan-out (SC1 data-plane half; VERIFY + GUARD, not a removal)

Verified in code (decisions-ledger): the node-facing channel already carries only
`ProjectSync`/`NodeRemoved` (+ per-node control/own-assignment/own-backfill), NOT shared-task state.
Phase-6 asserts and fences that invariant against regression; it is NOT a large fan-out removal.

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 601 | No-fanout invariant guard — exhaustive `HiveMessage` classification + topology assertion (hermetic, no DB) | dep: 103 202 501 | conflicts: none | SC1 |
| 602 | No-fanout send-site comment fence — document the SC1 invariant at `connection.rs` | dep: 601 | conflicts: none | SC1 |

> **601 `depends_on: 103` (the enum-drift fence).** Phase-6 is sequenced after P1/P2/P5, which ADD
> hive→node `HiveMessage` variants (`OpAck`@P1-task-103, `LeaseGrant`/`LeaseRevoked`@P2,
> `DigestResult`@P5 — CONTRACT §A). 601's classification `match` is exhaustive over `HiveMessage`, so it
> must see the GROWN enum, not main's. The dep on the authored variant-adding task **103** makes the
> ordering explicit; the P2/P5 variants (whose tasks are not yet authored as ids) are **decision-locked**
> in 601's body (a fixed `variant → Delivery` table the executor applies — none is fan-out), so the guard
> never devolves to executor judgment when those variants land.

### Phase 7 — hive-only-state cutover (SC6 / TS6; ADR-0011 migrate / regenerate / discard)

**In-place rebuild — judgment call ratified at authoring (NEEDS ORCHESTRATOR/USER RATIFICATION).** No
task in this workstream rebuilds the hive schema; every REGENERABLE/DISCARDABLE table still has surviving
`query!` refs in `crates/remote/src` (code removal is P4/P5, out of scope here), and the node re-ingest
path `INSERT`s into the existing table (`node_task_attempts.rs:52`). So the cutover is a **DATA operation
(TRUNCATE/DELETE), NOT a schema DROP**: MUST-MIGRATE tables (incl. the `source_task_id`/`source_node_id`
id bridge) stay in place, and the hive `task_status` enum is already canonical kebab-case (the
`inprogress`→`in-progress` remap is at the node→hive ingest boundary, not at rest). The destructive
alternative (copy MUST-MIGRATE into a fresh empty schema) is NOT authored; each task STOPs if it is
mandated instead.

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 701 | Cutover migration — data-clear REGENERABLE + DISCARDABLE hive-only state (in-place) | dep: - | conflicts: none | SC6 |
| 702 | Cutover guard — MUST-MIGRATE round-trip: id bridge intact + status canonical | dep: 701 | conflicts: none | SC6 |
| 703 | Cutover guard — REGENERABLE tables repopulate from a simulated node re-ingest | dep: 701 | conflicts: none | SC6 |

> **701 is `irreversible: true`** (data loss; gated behind a pre-cutover backup, ADR-0011) — needs a
> `reviews/701.approved` token before its gate runs. All three are Postgres + fail-closed (Trap 2b): the
> `## Done when` carries `test -n "$DATABASE_URL" && cargo test …` so a no-DB run fails instead of a
> hollow skipped green. 701's test is **seed → run the cutover SQL → assert** (not connect-and-count) so
> it is non-hollow even though the migration ran at `migrate!` time on an empty DB. 703 carries a
> ratified FIDELITY limitation: it drives the EXISTING `NodeTaskAttemptRepository::upsert` re-ingest path
> (the new ADR-0008 op-log re-ingest for attempts does not exist yet — P1 shipped only `task.upsert`),
> proving the schema is refillable post-cutover, not the op-log mechanism.
>
> **FROZEN-SPEC COLLISION (NEEDS RATIFICATION):** spec TS6 says "discardable tables are **absent**"; the
> in-place reading keeps them present-but-emptied (their code refs are removed only in P4/P5, and P7
> depends on P1–P3). Ratify keep-but-empty (option a, authored) OR sequence a follow-up DROP after the
> P4/P5 code-removal (option b). See the decisions-ledger Phase-7 item 2b.

### Phase 4 — inbound collapse (SC7 / TS5; authored this pass — ADR-0007)

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 401 | TS5 acceptance — single inbound channel, one delete, one conflict + topology guard | dep: 402 403 404 | conflicts: 402 403 | SC7 |
| 402 | One delete semantic — both inbound legs soft-unlink via a single shared helper | dep: - | conflicts: 401 403 404 | SC7 |
| 403 | Dirty-guard in `upsert_remote_task` — inbound never clobbers an unacked local edit | dep: 104 | conflicts: 401 402 | SC7 |
| 404 | Handle `task.reassigned` in the activity processor — no dropped event types | dep: - | conflicts: 402 | SC7 |
| 405 | Remove the dead `ElectricTaskSyncService` task-shape path (IRREVERSIBLE) | dep: - | conflicts: none | SC7 |

> **Phase-4 authoring notes (recorded in the ledger):** (i) the bulk-snapshot reconcile is ALREADY
> connect-gated (its only caller is the `HiveEvent::Connected` arm; no periodic task re-sync exists), so
> "demote to cold-start/gap-fill" is a **verify + guard** (401's comment fence + topology STOP-trigger),
> NOT a removal — mirrors the SC1 no-fanout "already satisfied" finding. (ii) A LATENT prod bug surfaced:
> the WS leg's `set_shared_task_id(.., None)` is a **no-op for a linked row** (SQLite `= NULL`
> three-valued logic, verified empirically), so 402 routes BOTH legs through one working
> `unlink_by_shared_task_id` and expands to touch `processor.rs`. (iii) Dirty-guard is **entity-level**
> (skip the whole apply when an unacked outbox op exists for the entity, predicate `acked_at IS NULL`) —
> strictly more conservative than the ADR's field-level wording; ratified judgment call. (iv)
> `task.reassigned` carries the identical `SharedTaskActivityPayload` as `task.updated`, so it routes
> through the same handler. **Shared files:** `sync.rs` (402+403), `processor.rs` (402+404),
> `node_runner.rs` (401+402) — encoded as `conflicts_with`. 403 has a cross-phase `depends_on: 104`
> (the P1 `OutboxRepository` file it extends with `has_unacked_for_entity`).

### Phase 3 — status machine (SC4; ADR-0010 reconciled, ratified matrix)

Explicit `task.status` single-author transition matrix over the REAL enum
(`Todo/InProgress/InReview/Done/Cancelled` — no `Assigned`/`Failed`; ADR-0010 §Decision, ratified
2026-06-30). Hive authors `Todo→InProgress` (assign+start), `InReview→Done`/`InReview→InProgress`
(operator review), `*→Cancelled`; the node authors `InProgress→InReview`/`InProgress→Done`, accepted
only with a valid lease + current fencing token. 303 enforces this in `handle_op_batch` (rides P2's
fencing seam — tasks 201/203/205 — as a prose+STOP precondition, not a `depends_on` edge). 304 closes the
SECOND node-status write site (legacy `handle_task_status`).

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 301 | Status transition matrix module — single-author guard table (ADR-0010) | dep: - | conflicts: 302 | SC4 |
| 302 | One canonical status wire value — node→hive mapping boundary helper | dep: 301 | conflicts: 301 303 304 | SC4 |
| 303 | Enforce single-author status transitions at `handle_op_batch` (node gated on lease+token) | dep: 301 302 205 | conflicts: 302 304 | SC4 |
| 304 | Route the legacy `handle_task_status` write through the transition guard | dep: 301 303 | conflicts: 302 303 | SC4 |

## Execution preconditions & closeout (READ — affects whether the gate passes)

- **Rust gate overrides:** every Rust task sets `WAI_TYPECHECK_CMD="cargo check -p <crate>"` (or
  `--workspace`) and `WAI_TEST_CMD="cargo test -p <crate> <test>"` in its `## Done when` (Trap 1).
- **SQLx offline:** tasks adding/changing a `query!`/`query_as!` export
  `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against a migrated dev DB; **no task runs
  `cargo sqlx prepare`** (Trap 2). The hive (Postgres) side has its own offline considerations — see
  the trap ledger's remote-crate analogue.
- **Closeout (NOT a gated task):** after all schema/query tasks land, regenerate the `.sqlx` cache once
  and commit it as a standalone housekeeping commit at `/wai:close`.

## SC coverage map (enforced ids SC1–SC7)

SC1→{phase-6 no-fanout (data-plane half); UI half carve-gated} · SC2→{phase-1} · SC3→{phase-2} ·
SC4→{phase-3} · SC5→{phase-5} · SC6→{phase-7} · SC7→{phase-4}. (Clause sub-ids SC2a/b/c, SC3 fencing,
SC5d-style negatives are mapped to specific tasks at authoring time.)
