---
topic: vk-swarm-hive-redesign
doc_type: decisions-ledger
---

# Decisions ledger — vk-swarm-hive-redesign

Appended during spec/precheck and (later) decompose/execute. Pre-empted traps for implementers are
added at decompose time.

## Pre-empted traps (read before executing ANY task)

Carried forward from `vk-swarm-node-foundations` (same repo, same gate) + hive/Postgres analogues. The
node-foundations ledger is the authority for the originals; condensed here.

### Trap 1 — WAI gate is TypeScript-shaped; this is a Cargo workspace
Every Rust task sets BOTH in its `## Done when`: `WAI_TYPECHECK_CMD="cargo check -p <crate>"` (or
`--workspace` when arms span crates) and `WAI_TEST_CMD="cargo test -p <crate> <test_name>"`. Omitting
them runs the wrong/no check — a hollow pass.

### Trap 2 — SQLx OFFLINE on the node (SQLite) side; build against a live migrated DB
Committed `.sqlx` cache + unset `DATABASE_URL` → `query!` validates against the cache, not a live DB.
Tasks adding a migration + new query export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against
a migrated dev DB. **Never run `cargo sqlx prepare` in a gated task** (it rewrites tracked `.sqlx/*.json`
the gate's file-allow-list rejects). Regenerate once at `/wai:close`.

### Trap 2b — Hive (Postgres / `crates/remote`) query validation + tests need a LIVE Postgres (confirmed)
Confirmed in code: the remote crate's `query!`/`query_as!` validate against the **shared root `.sqlx`
offline cache** (same cache as the node side). Adding/changing a Postgres query therefore needs a live
`DATABASE_URL`/`SERVER_DATABASE_URL` pointed at a **migrated Postgres** (`crates/remote/src/config.rs:45`)
OR a cache regen — it will NOT validate against the SQLite dev DB. **And hive integration tests SKIP
without a DB:** `crates/remote/tests/backfill_e2e.rs` uses `skip_without_db!` (returns early if
`DATABASE_URL` unset); the hermetic SQLite `db::test_utils::create_test_pool()` does **not** apply to
the hive. **Consequence for every hive-side task (102, 107, and all of P2/P3/P6/P7 hive work):** its
`scope_test` is a `#[tokio::test]` that REQUIRES `DATABASE_URL=postgres://…` against a migrated PG, or
the WAI gate runs a SKIPPED test and reports a **hollow pass**. The executor MUST stand up a Postgres
(migrated via `sqlx::migrate!("./migrations")`, `crates/remote/src/db/mod.rs:51`) and export
`DATABASE_URL` before running any hive task. A hive task with no Postgres precondition + a skip-guarded
test is NOT verifiable — flag such tasks. (This is genuinely harder than node-foundations, which never
touched the remote crate. If a CI Postgres is not readily available, raise it before executing P-hive
tasks.)

### Trap 3 — `enum_dispatch` / WS-message exhaustiveness on BOTH ends
Adding a `NodeMessage` or `ServerMessage` variant (op-log op, durable ack, heartbeat, lease grant)
forces editing **every** match arm on that end in the SAME commit or the workspace won't compile under
`-D warnings`. The node side (`hive_client.rs`/`hive_sync.rs`/`node_runner.rs`) AND the hive side
(`crates/remote` WS dispatch) each have their own exhaustive match sites — a new variant is a single
cohesive `mixed` task per end (or one cross-cutting task), not a create+wire split. Anchor list of arms
is filled from the hive-side anchor investigation.

### Trap 4 — Know which crate owns the logic (no cross-crate inversion)
`services` does not depend on `local-deployment`; trait methods dispatch into the impl. Confirm the
owning crate/trait for each touched function before authoring (the node-foundations recovery work lived
in the `services` trait, not `local-deployment`). For hive logic, confirm `crates/remote` ownership.

### Trap 5 — Anchor-checker strips `crates/*/` (precheck false-positive) — already hit
See Precheck notes below. Recorded so a precheck re-run is not mistaken for a real contradiction.

### Trap 6 — Frozen spec; escalate scope discoveries to the user
The spec is frozen at precheck (ADR-0001). A spec-vs-reality contradiction or an entangled/oversized
area discovered during decompose is **escalated to the user**, never papered over with a vague task or
a silent spec edit — exactly how node-foundations carved out `vk-swarm-node-ui-localize`. The SC1
central-UI carve (see plan Scope note) follows this protocol.

## Decompose decisions (2026-06-30)

### USER-APPROVED SCOPE SPLIT — SC1 central-management UI → `vk-swarm-hive-ui`
Decompose found SC1 has two separable halves: (a) the **data-plane** guarantee "no node↔node /
node↔hive↔node fan-out", and (b) the **hive web UI manages all** cross-node tasks/attempts/executions.
Half (b) is a workstream-scale frontend build (the hive UI today is a 4-page auth stub; the
management UI lives in the *node* frontend; cross-node task/attempt/execution views are net-new). **User
decision (2026-06-30): carve half (b) into `vk-swarm-hive-ui`** (tracker seeded, no spec yet). This
workstream covers SC2–SC7 + SC1's data-plane half (phase-6). SC1 stays in the frozen spec, claimed by
phase-6; the UI deferral is recorded here — **no spec edit** (mirrors the node-foundations →
`vk-swarm-node-ui-localize` carve).

### Finding — SC1 "no fan-out of shared task state" is LARGELY ALREADY SATISFIED
Verified in code: the hive's node-facing broadcast (`ConnectionManager::send_to_nodes`/`broadcast_to_org`,
`crates/remote/src/nodes/ws/connection.rs:138,189`) carries **only** `ProjectSync` and `NodeRemoved` —
it does **not** push shared-*task* state to nodes today. The shared-task fan-out that exists
(`electric_proxy.rs` `GET /v1/shape/shared_tasks` + `ActivityBroker`, `crates/remote/src/activity/broker.rs:48`)
targets **browsers** (which the hive UI needs). **Therefore phase-6 (SC1 data-plane) is "verify + guard
against regression + keep the node-facing channel assignment-only," not "remove a large fan-out."** This
is surfaced (not a contradiction): SC1's data-plane intent is met by construction; the task asserts and
fences it. Browser-facing fan-out (electric_proxy/broker) is OUT of scope for "no fan-out" — it is the
hive-UI data source.

### Decompose decision — WS protocol enums are hand-duplicated; DUAL-EDIT, do not deduplicate (this workstream)
`NodeMessage`/`HiveMessage` are defined **twice** — hive `crates/remote/src/nodes/ws/message.rs:15,91`
and node `crates/services/src/services/hive_client.rs:82,123` — and have **already drifted** (`Deregister`
exists hive-side only). Every new variant (op-log op, durable ack, heartbeat, lease grant) is added to
**both** copies in the same commit (Trap 3). **Decision:** keep the dual-definition convention and add
variants to both — deduplicating into a shared protocol crate is hygiene OUT of scope here (it would be
a large cross-crate move touching every match site). Each protocol task is a single cohesive `mixed`
task editing both crates + the exhaustive hive match (`session.rs:512`) + an explicit node arm **before
the `_ =>` wildcard at `hive_client.rs:1062`** (the wildcard silently drops unhandled hive→node variants
— the #1 easy-to-miss bug here).

### Decompose decision — durable-ack emission requires threading `ws_sender` into apply handlers
Hive apply handlers `handle_attempt_sync`/`handle_execution_sync`/`handle_logs_batch`
(`session.rs:1188,1307,1388`) currently take `node_id`/`pool` only — **not** the WS sender. Emitting a
durable per-op ack (the SC2c fix) requires threading `ws_sender` into those signatures. `handle_task_sync`
(session.rs:1547, replies `TaskSyncResponse`) and `handle_heartbeat` (replies `HeartbeatAck`,
session.rs:607) are the existing templates. The op-log apply is a NEW `handle_op_batch` handler beside
them; acks reuse the `HeartbeatAck` send shape.

### Decompose decision — PHASE-BY-PHASE authoring (user-approved)
This decompose authors **Phase 1 (the op-log foundation) only**; later phases are authored in subsequent
`/wai:decompose` passes as each prior ships (user decision 2026-06-30). Consequence for the gate:
`wai-plan-lint` reads ALL spec SC ids from the token and will report SC1,SC3–SC7 **unclaimed** until
their phases are authored — this is **expected-pending**, not a lint failure to paper over. Phase 1's
internal consistency (plan↔frontmatter deps/conflicts, verification present, failing-tests-first) IS
enforced now. A full PLAN-LINT PASS is achieved only when the final phase is authored.

## Phase-1 authoring — ratified judgment calls + tracer limitations (2026-06-30)

The Phase-1 author surfaced six judgment calls (Trap 6). Ratified after review:

1. **106 parks on project-link-absent, not parent-task-absent** — RATIFIED. `upsert_from_node`
   (`tasks.rs:566`) has no parent-task FK; the real transient dependency is the ProjectsSync link, which
   `handle_task_sync` already encodes. Correct reading of SC2b at the task tier.
2. **106 splits PARK (transient, `node_local_projects` row absent) vs SKIP+ADVANCE (permanent,
   not swarm-linked)** — RATIFIED, and load-bearing. Because 105 enqueues for ALL projects (the `db`
   crate can't cheaply do the swarm lookup), a non-swarm task at the outbox head would park *permanently*
   and wedge the cursor if "not linked" parked. The split (encoded as 106's third test) prevents the
   wedge. This mirrors `handle_task_sync`'s three-branch resolution.
3. **105 idempotency_key = `task:{id}:{uuid}` (per-write-unique, persisted on the op row), NOT
   `task:{id}:{version}`** — RATIFIED. `Task::update` (`queries.rs:305`) bumps no version, so a
   version-key would self-collide and `UNIQUE(idempotency_key)` would silently drop updates. A re-sent
   op reuses its stored key → hive dedups. (Same no-version fact as ADR-0007's dirty-guard motivation.)
4. **105 enqueue is best-effort, non-atomic** with the task write (callers hold `&SqlitePool`, not a
   txn) — RATIFIED as a TRACER LIMITATION. The legacy sync path is the backstop. **Consequence:
   Phase-1-tracer does NOT fully satisfy SC2c "zero silent write loss" — true no-loss needs the enqueue
   in the SAME transaction as the entity write** (threading a txn through the ~8 `Task::create` callers),
   which is the next Phase-1 increment. SC2c is *claimed* by 102/106/108 (the durable-ack mechanism) but
   the no-loss guarantee is only fully closed by the transactional enqueue.
5. **108 advances the cursor via a new `HiveEvent::OpAck`** consumed in `run_node_runner` (which holds
   the pool), mirroring the `TaskSyncResponse`/`BackfillRequest` "DB write happens in run_node_runner"
   pattern — RATIFIED. `HiveEvent` has no third/cross-crate match site (grep-verified), so 108's `files:`
   (hive_client.rs + node_runner.rs) is complete.
6. **Trap 2b (Postgres) applies to 102 + 106** — RATIFIED; each states the live-PG precondition.

**Honest SC2 status after Phase-1-tracer:** the ordered, acknowledged round-trip *mechanism* is proven
(SC2a single ordered channel — additive, alongside legacy; SC2b parent/link-before-child parking; SC2c
durable per-op ack + cursor-advance-only-on-ack). NOT yet done: transactional enqueue (full SC2c
no-loss), attempt/exec/log op types, and retirement of the five legacy push paths. These are tracked as
the next Phase-1 increment in plan.md.

## Phase-1 sibling-advisory acknowledgement (wai-plan-lint `W:` lines, SC6)

- **101 `…_add_node_outbox.sql` beside `…_init.sql`** — migrations are independent forward-only DDL, NOT
  reimplementations of a pattern. Authored to house conventions confirmed against the recent
  `queued_messages` migration (BLOB UUID PKs, `datetime('now','subsec')`, `CREATE … IF NOT EXISTS`,
  partial index). Not a pattern sibling.
- **102 `…_add_node_op_log.sql` beside `…_shared_tasks_activity.sql`** — same: independent Postgres DDL,
  not a pattern sibling. **102's test `node_op_log_migration.rs` beside `backfill_e2e.rs` IS a real
  sibling** — the task reads it and reuses its `database_url()`/`skip_without_db!`/`create_pool()`
  helpers verbatim (no shared `common` module exists). Acknowledged, handled in-task.
- **104 `node_outbox.rs` beside `activity_dismissal.rs`** — the task carries a `## Sibling alignment`
  step reading an existing `db` model (`draft.rs`) for trait surface / error type / test style. The
  genuine pattern sibling is read; `activity_dismissal.rs` is one of many same-dir models, not the
  authority. Acknowledged.

## Phase-1 lint status (expected-pending, not a failure to paper over)
After the Phase-1 fixes (SC2/TS1 claimed; clause sub-ids SC2a/b/c are colon-less PROSE in the spec so
the lint's declared id is `SC2`), `wai-plan-lint` reports only **SC1, SC3–SC7 and TS2–TS7 unclaimed** —
the unauthored phases. This is the documented phase-by-phase state; a full PLAN-LINT PASS lands when the
final phase is authored. Phase-1 internal consistency (deps/conflicts ↔ frontmatter, verification
present, failing-tests-first) passes.

## Phase-6 authoring — no-fanout VERIFY+GUARD (2026-06-30)

Phase-6 (SC1 data-plane) authored as a small verify+guard, per the plan/CONTRACT §F. Re-verified the
current no-fanout state directly before authoring; the invariant **HOLDS** — see findings below.

### Re-verification — every hive→node send site enumerated (the no-fanout state confirmed)
`grep "HiveMessage::"` across `crates/remote/src` + the four send primitives in `connection.rs`
(`send_to_node`@123, `broadcast_to_org`@138, `broadcast_to_org_except`@159, `send_to_nodes`@189) yields
the COMPLETE hive→node send inventory:
- `ProjectSync` (session.rs:856/994/1859 via `send_to_nodes`/`broadcast_to_org`) — project METADATA
  (link id, repo path, branch, owner-node display), NOT shared-task state. Targeted to linked nodes.
- `NodeRemoved` (session.rs:1087 via `broadcast_to_org`) — node-lifecycle control.
- `TaskAssign` (dispatcher.rs:103/252 via `send_to_node`) — sent ONLY to the **target/owning** node;
  it IS that node's OWN assignment, NOT relay of another node's state. (Task-shaped, but not fan-out.)
- `BackfillRequest` (backfill.rs:188/257 via `send_to_node`) — a request for the recipient to push ITS
  OWN data up; carries no task state.
- `AuthResult`/`Close`/`HeartbeatAck`/`TaskSyncResponse`/`Error`/`StatusRequest` (session.rs, via the
  socket sink `send_message`, not the broadcast primitives) — per-node handshake/control/ack replies on
  the recipient's OWN socket.
- `LabelSync` (broadcast variant) — declared in `HiveMessage` but **never constructed/sent** today (the
  inbound `NodeMessage::LabelSync` handler ignores it; session.rs:543/1536). Classified as `LabelMetadata`
  defensively; even if sent it is org-global label metadata, not task state.
- `TaskCancel` — constructed ONLY in a commented-out TODO (dispatcher.rs:189-194); never sent.
**Conclusion (no finding):** no `HiveMessage` the hive delivers to a node pushes another node's shared-
*task* state to a third node. SC1's data-plane intent is met by construction; phase-6 asserts + fences it.

### Judgment call — anchor list in the task brief was a SUBSET; the guard covers ALL send variants
The brief's anchors named ProjectSync@889/1021/1873 + NodeRemoved@1096 as the call sites. The full
inventory above also includes `TaskAssign` (dispatcher) and `BackfillRequest` (backfill.rs) sent via
`send_to_node`, plus the per-node `send_message` replies. These are NOT fan-out of foreign task state
(own-assignment / own-backfill / per-node control), so they do not contradict §F — but the 601 guard
classifies **every** `HiveMessage` variant exhaustively (not just the two ProjectSync/NodeRemoved
broadcast payloads), which is strictly stronger and future-proof. Recorded so a reviewer expecting only
the two named variants understands the guard's wider, deliberate scope.

### Judgment call — HERMETIC test (no Postgres); Trap 2b deliberately NOT invoked
601 is a pure static/enum guard over `HiveMessage` — no `query!`, no socket, no DB. Per the brief
("a pure enum/static guard test may be hermetic — prefer that"), it is a plain `crates/remote/tests/`
integration test importing `vks_hive_server::nodes::ws::message::HiveMessage` (all `pub`). It has **no**
`DATABASE_URL` precondition and **no** `skip_without_db!` — so it can never report a hollow skip-pass.
This is the one phase-6 task that escapes Trap 2b (it touches no Postgres query). Gate uses the Rust
overrides (Trap 1): `cargo check -p vks-hive-server --tests` + `cargo test --test no_fanout_invariant`.

### Judgment call — 2 tasks: `create` guard (601) + comment-only `edit` fence (602)
Split to keep `allowed_change` clean: 601 is `create` (new test file, the executable invariant); 602 is
a comment-only `edit` to the SHARED file `connection.rs` (the documented regression fence at the send
sites, pointing future authors at the 601 test). 602 `depends_on` 601 (the comment references the test).
602 has `scope_test: N/A` + a `## Manual verification` section (comment-only; no behavior to unit-test) —
schema-valid. **SHARED FILE NOTE:** 602 touches `crates/remote/src/nodes/ws/connection.rs`, also the
home of the `send_to_node`/`broadcast_to_org`/`send_to_nodes` primitives that later phases (P2 lease,
P3 status) read but do not modify; 602's edit is the module doc-comment only (no signature/behavior
change), so it does not conflict with any P1/P2/P3 task — `conflicts_with: []` is correct.

### Judgment call — 602 claims NO `covers_criteria`/`covers_tests` (601 fully claims SC1+TS7)
602 is a comment-only `edit` to a CODE file (`connection.rs`). `wai-plan-lint`'s SC6 rule fires for any
`create|mixed|edit` task with a non-empty `covers_criteria` AND ≥1 code file, REQUIRING a non-empty
`## Failing test (write first)` free of `N/A`/deferral keywords; the doc-only carve-out applies only when
ALL `files:` are `.md/.mdx/.json/.toml/.yaml/.yml` — which a `.rs` edit is not. A comment fence has no
failing test to write. **Resolution:** 602 carries `covers_criteria: []` and `covers_tests: []` — SC1 and
TS7 are fully claimed by **601** (which DOES write the failing test). 602 is a supporting documentation
fence, not an SC-claiming task; this satisfies the lint and is the honest reading.

### Judgment call — fence centralized at `connection.rs` (the four send primitives), NOT `session.rs`
The brief named both files as send sites. EVERY hive→node push funnels through the four `ConnectionManager`
primitives in `connection.rs` (`send_to_node`/`broadcast_to_org`/`broadcast_to_org_except`/`send_to_nodes`);
`session.rs`'s `send_message` is the per-node socket-sink reply path for handshake/control/ack
(auth_result/close/heartbeat_ack/task_sync_response) — NOT a broadcast/relay path, so it cannot fan out
another node's task state by construction. The 602 comment fence sits ONLY at `connection.rs` (the choke
point; lower-conflict, doc-comment only); **601's exhaustive `HiveMessage` guard covers the
`session.rs`-delivered variants too** (it classifies ALL variants regardless of sending primitive). The
`session.rs` omission is intentional.

### Judgment call — ENUM-DRIFT fence: 601 anticipates the GROWN `HiveMessage` (P6 runs after P1/P2/P5)
601's `classify` is an EXHAUSTIVE match over `HiveMessage`; P6 is sequenced after P1/P2/P5, which ADD
hive→node variants (CONTRACT §A: `OpAck`@P1-task-103, `LeaseGrant`/`LeaseRevoked`@P2,
`DigestResult`@P5-task-501). Against main's 12-variant enum the authored match goes NON-exhaustive once
those land → compile error → gate fail, leaving the executor to classify them by judgment (the exact drift
WAI decision-locking prevents). **Fixes:** (1) 601 `depends_on: ["103"]` (plan row `dep: 103` for lint
equality) — the authored variant-adding task; 601's test code already includes the
`OpAck { applied_through_seq }` arm + constructor (guaranteed via the dep), classified `PerNodeControl`.
(2) A DECISION-LOCKED table in 601's body fixes `LeaseGrant`/`LeaseRevoked`→`OwnAssignment`,
`DigestResult`→`PerNodeControl` (all struct-variants `{ .. }` per §A; `DigestResult` confirmed against 501)
— the executor APPLIES the rule, never judges. None is foreign-task-state fan-out, so the assertion holds.
P2/P5 are NOT hard task-id deps (their ids aren't authored yet; the spine ordering is prose) — the
decision-locked table + commented arms cover them when they land.

### Lint note (expected-pending) — SC1 + TS7 CLAIMED by phase-6 (601)
With phase-6 authored, `wai-plan-lint` SC1 moves unclaimed → claimed by **601** (`covers_criteria: [SC1]`,
`covers_tests: [TS7]`); 602 claims neither. Remaining unclaimed ids are the still-unauthored phases
(2,3,4,7 → SC3/SC4/SC7/SC6 + TS2/TS3/TS5/TS6; P5 authored separately) — the phase-by-phase state, not a
failure.

## Phase-5 authoring — anti-entropy digest, SC5/TS4 (2026-06-30)

Phase-5 (anti-entropy reconciliation digest) authored as four tasks (501–504) riding P1's op-log per
ADR-0008 §"Anti-entropy reconciliation (SC5)" + CONTRACT §A. Mirrors P1's 103/107/106/108 shape
(protocol variants / node emit / hive apply+reply / node act). Judgment calls below were ratified before
authoring (advisor-reviewed).

### CONTRACT §A shapes are FIXED — digest carries NO hash, NO outbox high-water field
The brief's prose ("per-entity version/hash + outbox high-water") is WIDER than CONTRACT §A's frozen
shapes: `DigestEntry = { entity_type, entity_id, version }` (no hash); `Digest = { entries }` (no
high-water field); `DigestResult = { resend_from_seq: Option<i64>, pull_entities: Vec<Uuid> }`. 501
implements the §A shapes EXACTLY and does not widen them — a hash/high-water field would be a Trap-6
CONTRACT divergence requiring escalation first. **This still satisfies TS4:** the hive already knows its
own cursor (`MAX(seq) FROM node_op_log WHERE node_id=$1`), so it does not need the node's high-water on
the wire; and TS4's two divergences are **set-difference on `entity_id`** (via the
`source_node_id + source_task_id` id bridge), not version comparisons. Recorded as a deliberate
narrowing-to-contract, not an omission.

### Heal-on-EXISTENCE, not version equality (the noisy-version trap)
`Task::update` bumps no local version (Phase-1 ledger judgment #3); `remote_version` is the hive's echoed
value (0 pre-ack vs hive `version:1`), so version diffs are noisy for node-owned tasks and would thrash a
version-gated heal. 503 drives `resend_from_seq`/`pull_entities` purely off entity EXISTENCE
(set-difference on the id-bridge key), NOT version mismatch. `version` is carried in `DigestEntry` for
future drift use but does not trigger heal in this tracer. STOP triggers in 503/504 enforce this.

### `resend_from_seq` requires a NEW `peek_from_seq` that IGNORES `acked_at` (504, +node_outbox.rs)
A node-has/hive-lacks divergence means an **acked-but-lost** op (the hive applied then lost it, or the
ack raced a crash). 107's `peek_unacked` filters `acked_at IS NULL` and therefore cannot reach such an
op. 504 adds `OutboxRepository::peek_from_seq(pool, from_seq, limit)` (no `acked_at` predicate;
`WHERE seq >= ?`) so acked rows replay. **Verified safe:** `mark_acked_through` only SETS `acked_at`
(104), and `grep -rn "DELETE.*node_outbox"` finds NO prune of outbox rows anywhere — acked ops persist
and are replayable. Over-resending is safe: 106's apply is idempotent (`ON CONFLICT DO NOTHING`) and
advances the ack only on a successful apply. This expands 504's `files:` to include
`crates/db/src/models/node_outbox.rs`.

### Pull leg reuses the EXISTING bulk-snapshot reconcile (no per-entity pull)
hive-has/node-lacks heals by re-running `sync_remote_projects` (node_runner.rs:941 — the reconcile
gap-fill leg ADR-0008/ADR-0007 name), not a new per-entity fetch. `pull_entities` being non-empty is the
TRIGGER to reconcile; the node lacks those local ids so it cannot look them up individually. The heal arm
lives in `run_node_runner` (which holds `db.pool`, `remote_client`, and `handle.state.node_id()`/
`organization_id()`), mirroring 108's `OpAck` seam (HiveClient is `&self`, holds no pool → emit a
`HiveEvent::DigestResult`, do the work in the loop).

### "On reconnect" folded into "first `sync_once` after reconnect" (no distinct hook)
There is no clean reconnect callback to anchor a digest send. The 30s `sync_once` loop (hive_sync.rs:131)
is the only driver; a reconnect re-establishes the command channel and the next tick emits a fresh
digest. 502 sends the digest at the tail of every `sync_once` (after 107's `sync_outbox`). No new
reconnect hook was invented (would expand `files:` out of scope). Recorded so a reviewer expecting an
explicit reconnect site understands the fold.

### TS4 acceptance lives in 503 (hive-side, fail-closed PG gate)
503 carries `covers_tests: [TS4]` and seeds the BIDIRECTIONAL divergence (node-has/hive-lacks +
hive-has/node-lacks) inside a `#[cfg(test)] mod` in `session.rs` (the handler is private — same as 106).
It REQUIRES a live migrated Postgres (incl. 102's `node_op_log`); the gate uses the
`test -n "$DATABASE_URL" && …` fail-closed prefix (Trap 2b) so a skip is a hard fail, not a hollow green.
The **"no `reset_*` manual step"** assertion is STRUCTURAL: convergence is produced purely by the
returned `DigestResult` (consumed by 504 + the reconcile leg) with NO test-side SQL repair — that IS the
SC5 contract. The hive compare also needs a new `SharedTaskRepository::list_source_task_versions_for_node`
(filtering `deleted_at IS NULL` so a tombstoned task is not resurrected, ADR-0007 one-delete), expanding
503's `files:` to include `crates/remote/src/db/tasks.rs`.

### Cross-phase SHARED-FILE collision with P2's lease protocol (CONTRACT §A) — not yet wireable
501 hand-duplicates `Digest`/`DigestResult` into BOTH `crates/services/src/services/hive_client.rs` and
`crates/remote/src/nodes/ws/message.rs` (+ exhaustive arms in `crates/remote/src/nodes/ws/session.rs`) —
the SAME two enum files P2's lease-protocol task edits (`LeaseHeartbeat`/`LeaseGrant`/`LeaseRevoked`).
CONTRACT §A: "your protocol task SHARES message.rs+hive_client.rs with P2's lease protocol task." P2 is
**not yet authored as task ids**, so 501's `conflicts_with` carries only the intra-P5 ids (503, 504) —
`wai-plan-lint` enforces plan-table==frontmatter equality but does NOT require referenced ids to exist,
so a forward reference to an unauthored P2 id would be a dangling-row risk. **Action for the P2 author:**
when P2 lands, its protocol task MUST add 501 to its `conflicts_with`, 501 is updated symmetrically, and
the two are `depends_on`-sequenced so they never edit the enum tails simultaneously (the dual-edit
convention, Trap 3). Also note 601 (phase-6) already decision-locks `DigestResult@P5` in its exhaustive
`HiveMessage` classification table — 501's variant is the one 601 expects.

### SHARED-FILE map across P5 (for cross-phase reviewers)
- `crates/services/src/services/hive_client.rs` — 501 (variants), 504 (HiveEvent + arm). Also P1
  103/108, P2 lease protocol.
- `crates/remote/src/nodes/ws/message.rs` — 501. Also P1 103, P2 lease protocol.
- `crates/remote/src/nodes/ws/session.rs` — 501 (stub arm), 503 (handler). Also P1 103/106.
- `crates/services/src/services/node_runner.rs` — 504. Also P1 108.
- `crates/services/src/services/hive_sync.rs` — 502. Also P1 107.
- `crates/db/src/models/node_outbox.rs` — 504. Also P1 104.
- `crates/remote/src/db/tasks.rs`, `crates/db/src/models/task/queries.rs` — 503, 502 respectively
  (new query methods beside existing ones).

### Lint note (expected-pending) — SC5/TS4 now CLAIMED by phase-5
With phase-5 authored, `wai-plan-lint` SC5 moves unclaimed → claimed (502–504, `covers_criteria: [SC5]`)
and TS4 → claimed (503, `covers_tests: [TS4]`). **501 carries `covers_criteria: []`** (not `[SC5]`) — it
is a compile-only protocol-stub with an N/A failing test, and the lint rejects a non-empty
`covers_criteria` paired with an N/A `## Failing test` (wai-plan-lint.sh:59-65); SC5 stays claimed by
502–504. This mirrors P1's 103 protocol-stub task exactly (`covers_criteria: []`, plan SC column
informational). Remaining unclaimed ids are the still-unauthored phases
(2,3,4,7 → SC3/SC4/SC7/SC6 + TS2/TS3/TS5/TS6) — the documented phase-by-phase state, not a failure.

## Phase-7 authoring — judgment calls (2026-06-30, NEEDS ORCHESTRATOR/USER RATIFICATION)

Phase 7 (hive-only-state cutover, SC6/TS6 per ADR-0011) authored as 701/702/703. The most
design-sensitive call (Trap 6) is surfaced for ratification.

1. **"Rebuilt hive" = IN-PLACE evolution, NOT a fresh-schema copy — RATIFICATION REQUESTED.** ADR-0011
   says "migrate … into the **rebuilt hive**." Two verified facts pin this to in-place: (a) no task in
   this workstream rebuilds the hive schema — P1/P2 add only ADDITIVE migrations (`node_op_log` table,
   `node_task_assignments` lease columns), nothing rebuilds `shared_tasks`; (b) the id bridge
   (`source_task_id`/`source_node_id`) already exists as columns on `shared_tasks`
   (`20260105120000_add_source_task_id.sql`) and is intact in place. The destructive alternative
   (DROP+recreate `shared_tasks`, copy MUST-MIGRATE data across a fresh empty Postgres schema) would be
   INVENTED and dangerous — NOT authored. **If the orchestrator/user wants a fresh-schema copy, 701/702/
   703 must be re-authored** (each carries a STOP trigger for this).
2. **Cutover is a DATA operation (TRUNCATE/DELETE), NOT a schema DROP — load-bearing correction.** The
   first draft of 701 DROPped the REGENERABLE/DISCARDABLE tables. Advisor + grep caught this: EVERY one
   of those tables still has surviving `query!`/`query_as!` refs in `crates/remote/src` (`node_task_attempts`
   13, `node_local_projects` 22, `activity` in `db/activity.rs`+`db/tasks.rs`, `auth_sessions` 8, …), and
   NONE of that code is removed by THIS workstream (the removal lives in P4/P5, out of scope here). An
   in-place `DROP TABLE` would (a) break `cargo check -p remote` online query validation, and (b) leave
   no table for the node re-ingest path (`handle_attempt_sync` → `NodeTaskAttemptRepository::upsert`, an
   `INSERT INTO`, NOT a `CREATE TABLE`) to repopulate. So 701 TRUNCATEs data and keeps schema.
2a. **701's test is seed→run-cutover-SQL→assert (non-hollow).** A naive "connect + assert row_count==0"
   test would PASS on a fresh migrated PG even with an EMPTY migration body (nothing was ever inserted) —
   the hollow-test class (tournament axis 7) reached via the migrate-on-empty-DB mechanism rather than
   the skip guard. 701's test instead SEEDs rows (regenerable + discardable + must-migrate incl. an
   ACTIVE and a COMPLETED assignment), re-runs the exact cutover statements (`CUTOVER_SQL`,
   copy-identical to the migration body), then asserts cleared-vs-retained — catching a `TRUNCATE`
   silently reaching a must-migrate table and an over-broad `DELETE` on active assignments. CASCADE/
   RESTART IDENTITY are deliberately OMITTED (UUID PKs; a plain TRUNCATE that ERRORS on an external
   inbound FK is the SAFE failure mode, not a silent must-migrate delete).
2b. **FROZEN-SPEC COLLISION — TS6 "discardable tables are absent" vs in-place keep-but-empty (NEEDS
   RATIFICATION, Trap 6 / fidelity axis 9).** Spec TS6 literally says "discardable tables are absent."
   The in-place reading keeps them present-but-emptied (their `query!` refs are removed only in P4/P5,
   and P7 depends on P1–P3 not P4/P5). Surfaced explicitly — NOT resolved by silently rewording 701's
   assertion. Orchestrator/user must ratify (a) keep-but-empty satisfies TS6's "nothing silently lost"
   intent (+ a spec note), OR (b) sequence a follow-up DROP migration AFTER the P4/P5 code-removal phase.
   701 is authored for (a); 702 has no DISCARDABLE assertion; switching to (b) adds a later DROP task.
3. **Status enum needs NO remap of stored hive rows.** The hive `task_status` enum is ALREADY canonical
   kebab-case `in-progress`/`in-review` (`20251001000000_shared_tasks_activity.sql:57`;
   `crates/remote/src/db/tasks.rs:24`). The `inprogress`/`inreview`→kebab remap happens at the node→hive
   INGEST boundary (op-log apply), so hive data-at-rest is already canonical. 702 therefore asserts the
   value space is canonical (kebab accepted, node-lowercase rejected) + a round-trip preserves it, rather
   than "remapping" existing rows.
4. **703 fidelity limitation (RATIFIED tracer-honesty).** The NEW ADR-0008 op-log re-ingest for
   attempt/exec/log op types does NOT exist yet — P1 shipped only `task.upsert` (see "Tracer honesty"
   above). So 703's "simulated re-ingest" drives the EXISTING `NodeTaskAttemptRepository::upsert` path
   (what `handle_attempt_sync` uses today), proving the cutover leaves a refillable schema — NOT proving
   the op-log mechanism. Stated in the test doc-comment.
5. **Already-removed tables NOT re-listed.** `node_projects` was dropped by `20260124200000`;
   `project_activity_counters` was dropped then RECREATED in `20260124100000` (so it currently EXISTS and
   IS cleared). The clear inventory was grounded against the live post-migration schema.
6. **Trap 2b (Postgres) applies to all three** — each states the live-PG precondition + the `test -n
   "$DATABASE_URL" &&` fail-closed prefix. **701 is `irreversible: true`** (data loss) — gated behind
   the ADR-0011 pre-cutover backup + a `reviews/701.approved` token.

### Lint note (expected-pending) — SC6/TS6 now CLAIMED by phase-7
With phase-7 authored, `wai-plan-lint` SC6 moves unclaimed → claimed (701–703, `covers_criteria: [SC6]`)
and TS6 → claimed (701–703, `covers_tests: [TS6]`). Remaining unclaimed ids are the still-unauthored
phases (2,3,4 → SC3/SC4/SC7 + TS2/TS3/TS5) — the documented phase-by-phase state, not a failure.

### Phase-7 sibling-advisory acknowledgement (wai-plan-lint `W:` lines)
- **701/702/703 tests beside `crates/remote/tests/backfill_e2e.rs`** — the genuine sibling. There is NO
  shared `common` module in `remote/tests/`; each task READS `backfill_e2e.rs` and inlines its exact
  `database_url()`/`skip_without_db!`/`create_pool()` helpers verbatim (same handling already ratified
  for Phase-1 task 102). Acknowledged, handled in-task — not an unread reimplementation.
- **701 migration beside `crates/remote/migrations/20251001000000_shared_tasks_activity.sql`** —
  independent forward-only Postgres DDL/DML (the cutover data-clear), NOT a reimplementation of the init
  schema. The real sibling for the data-clear shape is `20260124100000_remove_legacy_projects.sql` /
  `20260124200000_remove_node_projects.sql` (named in 701's Change section), read for the
  `DELETE FROM`/cleanup convention. Not a pattern sibling of the init migration.

## Phase-3 authoring — status state machine (SC4 / TS3) (2026-06-30)

Phase-3 authored as 4 hive-only (`crates/remote`) tasks: 301 matrix module (new `status_machine.rs`),
302 canonical wire value, 303 enforcement at `handle_op_batch` (TS3), 304 legacy-path guard. **P3 adds NO
WS variant** (CONTRACT §A: status rides the P1 op-log; Trap 3 does NOT apply). All four `covers_criteria
[SC4]`; only 303 carries `covers_tests [TS3]`.

### ⚠️ OPEN ESCALATION (Trap 6 / CONTRACT-header "record divergence HERE FIRST") — ADR-0010 §D names enum values that do not exist
**This BLOCKS execution of 301/303/304 and is recorded here per the CONTRACT header requirement before
any code.** ADR-0010 §D and CONTRACT §D name node-reported `in-progress→failed` and hive-authored
`todo→assigned`/`assigned→in-progress`/`assigned→todo`. **`Failed` and `Assigned` are NOT variants of
`TaskStatus`** — both enums (node `crates/db/src/models/task/mod.rs:27`, hive `crates/remote/src/db/tasks.rs:25`)
are exactly `Todo/InProgress/InReview/Done/Cancelled`. Conversely `InReview` exists in the enum and is
PRODUCED by the live node path (`Completed→InReview`, `session.rs:672`) but is absent from §D's node list.
So **§D is not encodable as written** — it references three non-existent values and omits one real one.
301 encodes a PROPOSED reconciliation (failures live in `execution_status`, not `task.status`; `assigned`
collapses into `InProgress`; node terminals are `Done`+`InReview`; hive authors `Todo→InProgress` and
`*→Cancelled`). This is a spec-vs-reality contradiction that **must be ratified by the user and recorded in
CONTRACT §D before 301/303/304 execute** (Trap 6 — never paper over). Open questions surfaced to the user:
1. Does `task.status` have no `failed` (failures tracked at the execution/assignment level)?
2. Is the node terminal `Done`, `InReview`, or both — and is `InReview→Done` hive- or node-authored?
3. Is hive "assigned" simply `InProgress` (collapsing `todo→assigned→in-progress`)?
4. Is operator-unassign `InProgress→Todo` legal? (The proposed matrix currently makes it illegal — a
   dropped §D row.)
5. Who authors `Running→InProgress` (a normal node "running" report on the legacy 304 path)? The proposed
   matrix marks `Todo→InProgress` Hive-authored, which would REJECT a node "running" report on 304 — a
   concrete symptom that dissolves once the matrix is ratified (do NOT patch 304 piecemeal first).
Each of 301/303/304 carries a top-of-body `## ⚠️ OPEN ESCALATION` block pointing here; 301 owns the
authoritative statement. **Once ratified, reconcile 301's matrix, 303's TS3 table, and 304's mapping, and
update CONTRACT §D + ADR-0010 to the agreed values.**

### Judgment call — two status-write sites; BOTH gated (advisor §3), legacy path may be P4-retired
The op-log path (106's `handle_op_batch` → `upsert_from_node`) is 303's site. A SECOND site writes
`shared_tasks.status` from a node: the legacy `handle_task_status` (`session.rs:625`) →
`update_status_from_node` (`tasks.rs:1009`), pure last-write-wins with no author/lease context (its
`Failed|Cancelled→Todo` map at `session.rs:673` is exactly the clobber ADR-0010 removes). Gating ONLY the
op-log path while claiming "SC4 no field-level conflict closed" would be a hole, so **304 routes the legacy
write through the same `status_machine` guard**. 304's STOP triggers note that P4 (inbound-collapse) may
instead DELETE the legacy path — if so 304 is superseded-obsolete (either outcome closes SC4's second
site). Surfaced rather than silently single-gating.

### Judgment call — fencing is RIDDEN, not re-authored (CONTRACT §C / P2 seam)
303's "node-reported transition accepted ONLY with valid lease+token" rides P2's existing fencing check
(CONTRACT §C: P2 adds the `op.fencing_token < assignment.fencing_token` reject to `handle_op_batch`). 303
adds the ORTHOGONAL transition-legality guard (this from→to is node-authored) and gates on P2's lease+token
decision; it does NOT re-implement the stale-token comparison. **P2's task id is unauthored**, so it is NOT
in 303/304 `depends_on` — the P2 lease+fencing dependency is a prose PRECONDITION + STOP trigger (mirrors
106's Trap-2b precondition; sanctioned by the phase-by-phase model). If P2 has not landed
(`node_task_assignments` lacks `fencing_token`/`lease_expires_at`, or `handle_op_batch` has no fencing
seam), 303/304 STOP. **Raised: P3 cannot fully execute until P2 ships.**

### Judgment call — matrix in a NEW file (`status_machine.rs`), hermetic test; TS3 in 303 (Postgres, fail-closed)
301's matrix is a pure enum/fn → its unit test is hermetic (NO `DATABASE_URL`, no fail-closed prefix) so
the author-legality table is never behind a DB skip. 302's canonicalization is likewise a hermetic
`&str→TaskStatus` test. 303 (TS3 accept/reject with lease+fencing context) and 304 (legacy DB write) are
Postgres-bound (Trap 2b) and use the FAIL-CLOSED `test -n "$DATABASE_URL" &&` gate prefix copied from 106.
The fail-closed prefix is applied ONLY to the two Postgres-bound tasks. New file `status_machine.rs` is the
only contention-free P3 artifact.

### SHARED FILE — `session.rs` (P1/106, P2, P3 all touch it)
302/303/304 all edit `crates/remote/src/nodes/ws/session.rs`, the same file 106 (P1) and P2 (fencing) edit.
302↔303↔304 `conflicts_with` each other (session.rs); 301↔302 `conflicts_with` (both edit
`status_machine.rs` — 301 creates, 302 extends). The P2 fencing edit to `handle_op_batch` and P3's status
guard touch the SAME apply block — 303 explicitly names the seam (insert the author guard between 302's
status-mapping and 106's `upsert_from_node`, gating on P2's lease/fencing decision) to avoid colliding with
P2's edit. P2's task ids are unauthored so they cannot appear in P3 `conflicts_with` (same dangling-id
constraint as the §C precondition) — recorded here.

### Lint note (expected-pending) — SC4/TS3 now CLAIMED by phase-3
With phase-3 authored, `wai-plan-lint` SC4 + TS3 move unclaimed→claimed (301–304 `covers_criteria [SC4]`;
303 `covers_tests [TS3]`). The lint collects `covers_criteria`/`covers_tests` from EVERY phase task file
with **no `status` filter** (`wai-plan-lint.sh:105` iterates all `phase-*/*.md`), so setting 301/303/304
to `status: blocked` (matrix unratified — see escalation above) does NOT unclaim SC4/TS3; the coverage map
stays satisfied while `/wai:next` correctly will NOT serve the blocked tasks. **plan.md task-table rows for
phase-3 are NOT added by this AUTHOR-ONLY pass; they are an integration step** (the report carries the exact
rows to paste, matching the final frontmatter).

### Lint FALSE POSITIVE — deferral-regex matches the `Todo` enum variant (cross-phase; analog of Trap 5)
`wai-plan-lint` SC6 greps the `## Failing test` body `-qiE '(…|TODO|FIXME)'` (`wai-plan-lint.sh:64`,
case-INSENSITIVE) to catch placeholder `TODO`/`FIXME` test stubs. The domain enum variant `TaskStatus::Todo`
and the wire string `"todo"` — UNAVOIDABLE in a status-state-machine test — match `TODO` as a substring, so
301/302/303/304 are flagged "Failing test defers" **falsely**. Their tests are REAL (table-driven matrix
assertions, not stubs). This is a tooling false positive on a legitimate enum name, the lint-side analog of
Trap 5's precheck path-anchor truncation FP. **Phase-4's 403 hits the IDENTICAL `TaskStatus::Todo` FP**
(confirmed) — it is cross-phase and pre-existing, not introduced here. Do NOT mangle valid Rust
(`TaskStatus::Todo` is the real variant) to placate the regex. Resolution options for `/wai:close`: tighten
the lint to word-boundary/uppercase-only `\bTODO\b|\bFIXME\b` (so it stops matching `Todo`), or accept the
documented FP. Recorded so a lint re-run is not mistaken for a real "deferred test" defect.
If `wai-plan-lint` enforces plan↔frontmatter row equality, those rows must be added at integration time.

## Phase-4 authoring — inbound collapse (SC7 / TS5, ADR-0007) (2026-06-30)

Tasks 401–405. Five surgical tasks; all `covers_criteria [SC7]`; 401 is the consolidated TS5 acceptance
(`covers_tests [TS5]`). Judgment calls surfaced (Trap 6) — ratify on integration:

### Judgment call — "demote bulk snapshot to cold-start" is VERIFY+GUARD, not a removal (RATIFIED in plan)
The bulk-snapshot reconcile (`sync_remote_project_tasks` → `fetch_bulk_snapshot`) is invoked from EXACTLY
ONE site — the `HiveEvent::Connected` arm of the node-runner loop (`node_runner.rs:677`). There is **no
periodic task re-sync**: the only `tokio::time::sleep` in the file is the reconnect backoff
(`node_runner.rs:1874`), and `spawn_hive_sync_service` (`:658`) syncs attempts/executions/logs, NOT
tasks. So the snapshot is ALREADY cold-start/gap-fill, NOT a second continuous channel. ADR-0007's "demote
to reconcile-only" is therefore met by construction; P4 makes it a **comment fence + topology STOP-trigger**
(task 401), exactly mirroring the SC1 no-fanout "largely already satisfied → verify + guard" finding. A
"demote the snapshot" task would have a no-op diff (hollow). **No spec edit** — SC7's "collapsed to one"
is delivered by the delete/conflict/event-type fixes + the guard.

### Finding — LATENT PROD BUG: the WS leg's soft-unlink is a NO-OP for a linked row
`process_task_deleted_event` (`processor.rs:437`) calls `set_shared_task_id(.., None)` by LOCAL id. That
fn's WHERE is `id=$1 AND (shared_task_id IS NULL OR shared_task_id=$2) AND NOT EXISTS(…=$2…)`. With
`$2 = NULL`: `shared_task_id = NULL` is never true (SQLite three-valued logic), so for a LINKED row
(`shared_task_id = S`) the predicate is false → **0 rows updated → never unlinked**. Verified empirically
(`UPDATE t SET s=NULL WHERE id=1 AND (s IS NULL OR s=NULL) …` leaves `s='S'`). So ADR-0007's premise "the
WS path soft-unlinks" describes code that does NOT unlink a linked task. **Consequence:** task 402 routes
BOTH legs through ONE working, executor-generic `unlink_by_shared_task_id` (`UPDATE … WHERE shared_task_id
= ?`) — "applied identically on both legs" by construction (ADR-0007 §2). 402 therefore EXPANDS to touch
`processor.rs` (not just `sync.rs` + `node_runner.rs`). This is a code-reality discovery achieving the
spec's stated intent, so NO re-precheck — but it is a real latent bug worth calling out at integration.

### Judgment call — dirty-guard is ENTITY-LEVEL (skip whole apply), not field-level (RATIFIED)
ADR-0007 §3 says "never overwrites a field that has an unacked outbound op". The node_outbox carries no
per-field dirty tracking — only per-entity ops (`entity_id`). Task 403's guard is therefore entity-level:
if `find_by_shared_task_id → local.id` has ANY unacked op (`acked_at IS NULL`), the WHOLE inbound apply is
skipped. This is strictly MORE conservative than field-level (it overwrites nothing while dirty) and fully
satisfies TS5's "concurrent local edit not clobbered". Placed INSIDE `upsert_remote_task` (the one fn both
legs funnel through) so the guard is identical on both legs by construction. Field-level granularity would
need a per-field dirty model the outbox doesn't carry — escalate if literal field-granularity is required.

### Judgment call — `task.reassigned` routes through the EXISTING upsert handler (RATIFIED)
The hive emits `task.reassigned` with the IDENTICAL `SharedTaskActivityPayload { task, user }` as
`task.updated` (`crates/remote/src/db/tasks.rs:982` → `insert_activity` builds the same payload; the new
assignee is a field on `task`). Task 404 adds `"task.reassigned"` to the existing
`"task.created" | "task.updated"` match arm — no separate handler (which would duplicate
`process_task_upsert_event`).

### Electric removal (405) is CLEAN — verified, irreversible-flagged
`ElectricTaskSyncService::sync_project_tasks` has ZERO runtime callers (grep: only doc-comments +
`pub mod`). `extract_uuid_from_key` is used only by the file's own `#[cfg(test)]`. The SEPARATE
`electric_sync` NDJSON client (mod.rs:7/57) STAYS — only `electric_task_sync` (the dead task-shape poll)
is removed. The integration test `crates/services/tests/electric_task_sync.rs` imports `electric_sync`
(the surviving client), NOT the deleted module — so it does not break. 405 is `irreversible: true`
(whole-file delete) + `forbid_after: [electric_task_sync, ElectricTaskSyncService, sync_project_tasks]`;
needs `reviews/405.approved`.

### SHARED-FILE map across P4 (for cross-phase reviewers)
- `crates/db/src/models/task/sync.rs` — 402 (add `unlink_*` helpers), 403 (dirty-guard in
  `upsert_remote_task`), 401 (TS5 test). 402↔403 `conflicts_with`; 401 sequenced after both via depends_on.
- `crates/services/src/services/share/processor.rs` — 402 (WS-leg unlink repoint), 404 (reassigned arm).
  402↔404 `conflicts_with`.
- `crates/services/src/services/node_runner.rs` — 402 (reconcile-leg repoint), 401 (comment fence).
  Sequenced (401 depends_on 402).
- `crates/db/src/models/node_outbox.rs` — 403 adds `has_unacked_for_entity` (cross-phase: P1 task 104
  creates this file; 403 `depends_on: 104`).

### Lint note (expected-pending) — SC7/TS5 now CLAIMED by phase-4
With phase-4 authored, `wai-plan-lint` SC7 + TS5 move unclaimed→claimed (401–405 `covers_criteria [SC7]`;
401 `covers_tests [TS5]`). plan.md phase-4 task-table rows ARE added by this pass (the inline edit) to
keep plan↔frontmatter row-equality green. After phase-4, only SC1 (carved UI half) + the not-yet-authored
phase-2 SC3/TS2 remain pending per the phase-by-phase plan.

## Phase-2 authoring — lease / atomic-checkout + fencing (SC3 / TS2, ADR-0009) (2026-06-30)

Phase-2 authored as 10 tasks (201–210), all `covers_criteria: [SC3]`; 210 alone `covers_tests: [TS2]`.
Anchors verified against current `main`. Judgment calls + reconciliations below (ratification welcome):

1. **`NodeTaskAssignment` FromRow column trap → narrow queries, struct NOT widened (203).** Adding
   `lease_expires_at`/`fencing_token` to the `#[derive(FromRow)]` `NodeTaskAssignment` (domain.rs:155) would
   break all ~9 existing `task_assignments.rs` SELECTs that omit them — sqlx `FromRow` `try_get`s every
   field at RUNTIME (`ColumnNotFound`), `cargo check` stays green, the gate's tests go red. **Decision:**
   201 adds the columns to the table only; 203/205/209 read them via narrow `RETURNING`/`query_scalar` into
   a purpose `LeaseClaim` struct. The repo file uses **runtime** `sqlx::query_as::<_,T>`/`sqlx::query` (NOT
   the `query!` macro), so the typecheck does not need the offline cache — only the tests need live PG. If a
   later phase genuinely needs the columns on the struct, that is its own task editing all 9 column lists.

2. **Node fencing-token stamp is STREAM-TIME, not enqueue-time — prompt/contract divergence (207).** The
   decompose prompt framed 207 as stamping at "the outbox enqueue path." That is impossible-and-wrong-by-ADR:
   the enqueue lives in `crates/db` (`task/queries.rs`, task 105), which has NO lease knowledge, and ADR-0009
   §3 says the node stamps the token **it believes it holds at SEND time** (the stale token for a partitioned
   node; `None` for node-owned work). The only place a node op is materialized onto the wire is `sync_outbox`
   (P1/107, `hive_sync.rs`). **Decision:** 207 stamps there from the live lease; `node_outbox.fencing_token`
   (col) stays NULL. This is a clean seam, NOT a user escalation.

3. **Lease state lives on `ActiveAssignment`, not a standalone `LeaseStore` (206/207/208 reconciled).** The
   stamp (207) must look up the token BY `local_task_id`, but `LeaseGrant` carries only `assignment_id`. The
   node already tracks `active_assignments: HashMap<assignment_id, ActiveAssignment{ …, local_task_id }>` on
   the shared `Arc<RwLock<NodeRunnerState>>` (node_runner.rs:257). **Decision:** add `fencing_token:
   Option<i64>` + `lease_expires_at: Option<DateTime<Utc>>` as fields on `ActiveAssignment`; 206 writes them,
   207 reads them (one map lookup), 208 reads `lease_expires_at` for the watchdog. One structure serves all
   three — no extra Arc plumbing. 206 owns the field definitions; 207/208 only read (conflicts_with 206 on
   node_runner.rs).

4. **`HiveSyncService.node_state` is `Option`, defaulting `None` (207 back-compat).** A required new ctor
   param would break P1/107's test + every existing `HiveSyncService::new` caller. **Decision:** add an
   `Option<Arc<RwLock<NodeRunnerState>>>` field + a `with_node_state` builder; `None` = passthrough (the 107
   behavior); only the live node spawn passes `Some`. Keeps 207 from rippling into P1.

5. **Self-fence reuses the existing kill, fences on EXPIRY + explicit revoke (208).** ADR-0009 §4 reuses the
   ADR-0001 process kill — `AssignmentHandler::handle_cancellation` (assignment_handler.rs:194) already does
   `stop_execution(.., Killed)`. **Decision:** the watchdog selects Running assignments whose
   `lease_expires_at < now` and invokes that halt; the `HiveEvent::LeaseRevoked` arm halts immediately.
   **Trap surfaced:** a freshly-assigned, not-yet-granted assignment also has `lease_expires_at = None` —
   the watchdog must NOT fence it. Resolved by fencing on EXPIRY (`Some(exp) < now`) only and letting the
   explicit revoke event fence directly — recorded in 208's STOP triggers for the executor to pin.

6. **203/205/209 specify the hard SQL as a CONTRACT (`todo!` + tests), not literal Before/After.** The
   atomic CAS (`try_claim`), the fencing reject control-flow, and the reclaim UPDATE interact with the live
   `idx_task_assignments_active` partial unique index and the available-vs-expired split, which must be
   finalized against the live schema. **Decision (deliberate, not an omission):** the method signatures +
   the per-method test set (4 for 203, 3 for 205, 2 for 209) pin the OBSERVABLE contract; the executor writes
   the statement that satisfies them. This is a known divergence from rubric #3's "exact Before/After"; the
   tournament will probe it — defensible because the behavior is test-pinned and the SQL is schema-dependent.
   (203's `LeaseClaim.lease_expires_at` is coordinated to `Option` so 209's reclaim, which NULLs it, fits the
   same struct.)

7. **TS2 proven as two coordinated legs, not a cross-process round-trip (210).** A real node binary ↔ real
   hive WS round-trip is not hermetically testable in one `cargo test`. **Decision:** 210 proves the hive
   stale-token REJECT leg (the at-most-once commit effect) against live PG; the node SELF-FENCE leg (bounded
   overlap) is proven by 206/208's hermetic unit tests, asserted by reference in 210 + recorded here. 210
   claims TS2 (the only task that does — required or `wai-plan-lint` hard-fails on unclaimed TS2).

**Trap-1/Trap-2b adherence:** every hive task (201,203,204,205,209,210) sets `WAI_TYPECHECK_CMD="cargo check
-p remote"` + a FAIL-CLOSED `WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote …'`, and its
PG-touching test is a `#[cfg(test)] mod` (private handlers) or a `tests/` integration test with inlined
`skip_without_db!`. Node tasks (206,207,208) set `cargo check -p services` (hermetic
`create_test_pool()`/in-memory — no PG precondition). 202 spans both crates → `cargo check --workspace`
(Trap-3 exhaustiveness on both ends).

### Cross-phase shared-file conflicts (orchestrator must reconcile)
These P2 files are ALSO edited by P1 (and later phases); declared within-phase conflicts do NOT capture the
cross-phase overlap — the orchestrator sequences/merges:
- `crates/remote/src/nodes/ws/message.rs` — P1/103 + P2/202 (WS enum dual-edit).
- `crates/services/src/services/hive_client.rs` — P1/103,108 + P2/202,206 (WS enums + HiveEvent).
- `crates/remote/src/nodes/ws/session.rs` — P1/103,106 + P2/202,204,205 (**205 EDITS 106's `handle_op_batch`
  → 205 `depends_on: 106`**, the single most important cross-phase dep).
- `crates/services/src/services/node_runner.rs` — P1/108 + P2/206,207,208 (HiveEvent arm + lease state).
- `crates/services/src/services/hive_sync.rs` — P1/107 + P2/207 (sync_outbox stamp).
- `crates/remote/src/db/task_assignments.rs` — P2-internal (203 + 205-reads + 209) — within-phase, declared.

### Phase-2 lint status
With Phase-2 authored, `wai-plan-lint` SC3 + TS2 move unclaimed→claimed (201–210 `covers_criteria [SC3]`;
210 `covers_tests [TS2]`). plan.md phase-2 rows ARE added by this pass (inline edit) for plan↔frontmatter
row-equality. Phases 1,3,4,5,6,7 + 2 are now all authored; the only remaining pending SC is **SC1's carved
UI half** (`vk-swarm-hive-ui`, deliberately out of this workstream). A full PLAN-LINT PASS should now land
modulo that documented carve.

## Cross-phase ratifications + tournament R2 + deployment runbook (2026-06-30)

### SC4 status machine — RATIFIED reconciliation (no spec edit / no re-precheck)
SC4's parenthetical `assigned`/`failed` are **authority labels, not `TaskStatus` values** (the real enum
is `{Todo, InProgress, InReview, Done, Cancelled}`, both crates). Ratified mapping: `assigned` = an active
`node_task_assignments` row (hive, assignment layer); `failed` = an `execution_status` outcome (node,
execution layer). The `task.status` matrix (hive: `Todo→InProgress`, `InReview→Done`/`InReview→InProgress`,
`*→Cancelled`; node, lease+token-gated: `InProgress→InReview`, `InProgress→Done`) is encoded in ADR-0010 +
CONTRACT §D. P3 tasks 301/303/304 were unblocked (`blocked→ready`) and reconciled. This corrects only my
own artifacts (ADR + CONTRACT), not the frozen spec — same class as the SC7 parenthetical and P4 prod-bug.

### P7 cutover — RATIFIED: in-place CODE + fresh-schema DEPLOYMENT RUNBOOK
User chose a fresh-schema rebuild (Q answered twice). Verified engineering reality: the DISCARDABLE
inventory items are **live infrastructure tables with code references** (`auth_sessions` 8 refs, `activity`
in `db/activity.rs`+`db/tasks.rs`, etc.) — NO schema (fresh or in-place) can omit them without breaking
`cargo check -p remote`, and `sqlx::migrate!` of a fresh DB recreates them **empty** (identical end state
to in-place). So the **code** cutover is the in-place data op (701–703, the only buildable form); the
**fresh-schema rebuild is this operational deployment runbook**:

> **Fresh-schema cutover runbook (deploy-time, not a code task):**
> 1. `pg_dump` the MUST-MIGRATE tables from the old hive (`node_api_keys`, `nodes`, active
>    `node_task_assignments`, `swarm_projects`+`swarm_project_nodes`, `swarm_templates`, `shared_tasks`
>    incl. the `source_task_id`/`source_node_id` id bridge, `labels`+`shared_task_labels`,
>    `users`/`organizations`/membership/`oauth_accounts`).
> 2. Create a fresh database; run `sqlx::migrate!("./migrations")` (builds the FULL schema — all tables,
>    incl. the live "discardable" infra ones, empty).
> 3. Restore the MUST-MIGRATE dump (apply the `inprogress`/`in-progress` status mapping on import if any
>    legacy rows predate the canonical kebab-case enum).
> 4. Bring nodes online — REGENERABLE state (`node_*` mirrors, logs, progress) refills via the op-log
>    re-ingest. DISCARDABLE data is NOT restored (tables present but empty).
> TS6 "discardable tables absent" is realized as **discardable DATA not retained** under both forms.

### Tournament R2 (Phases 2–6) — see `reviews/tournament-r2.md`
8 peer-validated findings remediated (codex+gemini; opencode DNF). Headline: a CRITICAL fencing bypass
(205 resolved the task by creator-node, breaking the fence for reassigned tasks → fixed to read
`payload.shared_task_id`), an SC5 replay-point bug (503 `MAX(seq)` missed older lost ops), a crate-name
compile error (601/602 used the bin name `vks-hive-server` not the package `remote`), and the cross-phase
WS-enum ordering edges (202↔501, 303→205, 601→{202,501}) the per-phase authors flagged for the orchestrator.

## Precheck notes

### Anchor-check false positive (resolved — `--no-anchor-check` used)
`wai-precheck.sh` assert #3 (path anchors) flagged `src/db/tasks.rs` and `src/activity/broker.rs` as
"ABSENT on main". This is a **false positive**: the extractor regex
`(src|extensions|ui|packages|apps)/…` truncates any `crates/*/src/*` path to its `src/…` substring and
tests it at repo root. The real files **exist on main** — verified directly:
`git cat-file -e main:crates/remote/src/db/tasks.rs` and `…/crates/remote/src/activity/broker.rs` both
succeed. These are the only two path tokens the regex extracts from this spec; both were manually
grounded. The sibling `vk-swarm-node-foundations` spec (shipped) produces the identical truncation
pattern. Precheck was therefore re-run with `--no-anchor-check`; the spec is frozen against
`spec_sha` in `.precheck.passed`.
