---
doc_type: spec
status: active
workstream: vk-swarm-event-bus
change_kind: behaviour
verify_cmd: "sqlite3 ${VK_DATABASE_PATH:-$HOME/.local/share/vibe-kanban/db.sqlite} \"select 1 from event_journal where event_type like 'task_%' limit 1\" | grep -q 1"
---

# vk-swarm-event-bus — task-lifecycle event bus (P4 / SC4)

## Intent
Phase 4 of the vk-swarm-refactor program (docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md). Owns umbrella success criterion refactor-SC4 (the parent program's SC4 — distinct from this spec's own SC4 defined under Success criteria below): task lifecycle changes emit events on an internal/external bus that downstream triggers consume. Independent of P3 (P3 is parallel to P4); together they gate P5-P7.

Give every node a durable, subscribable event bus so that things happening in vk-swarm (task changes, attempt/executor runs, hive connectivity) become first-class, ordered, replayable events instead of state that consumers must poll for. This is the nervous system the later phases attach to: P6's management agent reacts to bus triggers and P7 exposes bus observability over MCP/ACP. UI live-update is NOT part of this phase — the board already streams live over the existing WebSocket JSON-patch channel (see Out of scope).

Decisions settled at intent time (interview 2026-08-07): transport is an in-process bus bridged to a durable journal (no external broker); ship-scope event sources are task lifecycle, attempt/execution lifecycle, and sync/connectivity (git/PR events deferred); day-one consumers are an internal trigger-hook API (the P6 seam, proven with one real trigger) and an external SSE subscription endpoint; delivery is at-least-once with cursor replay — consumers must be idempotent.

## User stories
- **US1:** As the operator, when a task is created, moved, or deleted on my node, I expect that change captured as a durable, ordered event I can query and replay afterwards.
- **US2:** As an external tool, I can subscribe to the node's event stream, disconnect, and resume from my last cursor without missing events.
- **US3:** As a future management agent (P6), I can register a trigger hook that reliably fires on matching lifecycle events, surviving node restarts.
- **US4:** As the operator, I can observe hive connectivity and reconcile activity as ordered events for ops visibility.

## Success criteria
SC1: On a running node, creating, moving (status change), and deleting a task each produce a journaled event with a monotonic seq, observable via the subscription endpoint and queryable from the journal afterwards.
→ US1
SC2: Starting a task attempt and its terminal outcome (finished or failed) each emit events carrying task id, attempt id, and executor identity.
→ US3
SC3: Killing and restoring the hive connection on a running node emits disconnected, connected, and reconcile-completed events in order.
→ US4
SC4: An external client subscribing with no cursor receives live events; disconnecting, causing N events, and resubscribing with its last-seen cursor delivers every journaled event above the cursor in seq order, with duplicates allowed and no journaled event skipped. A hole in the seq integer sequence is NOT a missed event (see Design, "Seq semantics") — the journal is the authority.
→ US2
SC5: Events journaled before a node process kill are still replayable after restart; seq numbering continues without reuse or regression.
→ US2
SC6: A registered trigger hook demonstrably fires on a matching event on the live node (observable side-effect on task_status_changed), and after a node restart it resumes from its persisted cursor without losing the events in between.
→ US3
SC8: SC1, SC2, SC4, and SC6 all hold with the hive unreachable.
→ US2

## Users
Operators: durable, ordered visibility into task lifecycle, connectivity, and reconcile activity for ops.

P6 management agent (future): consumes trigger hooks to select ready work — the primary design customer for ordering/replay semantics.

External tools / P7 fabric (future): subscribe over the external endpoint to observe runs without touching internal state. P7 (MCP/ACP connectivity) is the named in-repo consumer of the SSE endpoint and lands in a later phase of the same program; this phase ships the contract ahead of it deliberately.

## Constraints
Reuse P2 patterns, do not invent: the durable ordered ack'd outbox precedent (crates/db/migrations/20260201000400_add_node_outbox.sql), local SQLite-as-journal, existing SSE/streaming serving patterns in the server crate (container.rs stream_raw_logs/stream_normalized_logs precedent), ApiResponse<T> routes, ts-rs typegen with manual registration in crates/server/src/bin/generate_types.rs.

Offline-first non-negotiable: the bus is fully functional with no hive; hive relay of events is out of scope (nodes already sync state up — do not duplicate that channel).

Bounded journal: the event journal must have a retention/compaction policy so it cannot grow unbounded on long-lived nodes (env-tunable per the VK_* convention in .env.example).

Typed events: the event schema is a Rust enum serialized snake_case, exported to TypeScript via ts-rs — one source of truth; no stringly-typed event names in consumers.

No breaking of streams that have consumers: current SSE/log-streaming endpoints with live consumers keep working. The pre-existing GET /api/events record-patch SSE route is the one exception and is explicitly in scope to delete — it has zero consumers repo-wide (only its own definition at crates/server/src/routes/events.rs and its mount at crates/server/src/routes/mod.rs:72 reference it), and its EventService backend is untouched by the deletion.

No SQLite writer-lock held across I/O: a journal write may share a transaction only with a discrete DB write statement, never with an enclosing function that performs git or filesystem I/O.

Reference systems read-only: paperclip for trigger/governance prior art.

GitHub targeting: PRs only against davidrudduck/vk-swarm.

## Out of scope
External message brokers (NATS/Redis/Kafka) — revisit only if multi-host fan-out beyond the hive ever demands it (ADR-0017 records the rejection).

Git/PR events (branch pushed, PR opened/merged) — deferred to a later increment.

Hive-side bus or cross-node event aggregation — the hive keeps consuming state sync; aggregating node buses is a later-phase concern.

Trigger logic beyond one proof-of-seam hook — rule engines, priority selection, and automation policy belong to P5/P6.

Exactly-once delivery — consumers are required to be idempotent instead.

UI live-update of any surface, including the board. The board already updates live without polling: frontend/src/pages/ProjectTasks.tsx:290 uses useProjectTasks, which for local projects streams JSON patches over /api/tasks/stream/ws (frontend/src/hooks/useProjectTasks.ts:80-92; route at crates/server/src/routes/tasks/mod.rs:66). refetchInterval polling applies only to remote swarm projects (useProjectTasks.ts:107). That channel is a read-model push (view state); this spec's bus is a domain event log (facts). Re-plumbing the board onto the event log would replace a working transport with one that fits view state worse — events carry lifecycle facts, not full task records, so the board would need a fetch per event or the event schema would have to fatten into records and lose its typed-lifecycle purity. Any future unification should drive the existing patch channel FROM the bus server-side, leaving browsers on view-state patches; that is a later-phase concern.

## Approach
Journal first, broadcast second (ADR-0017). Every covered lifecycle change writes a typed event row into a new event_journal table — in the same SQLite transaction as the discrete write statement that changes state — then publishes the committed event on an in-process tokio broadcast channel. Consumers get at-least-once delivery with no journaled event skipped by reading the journal from a seq cursor and then switching to the live channel. Two consumer surfaces ship: an internal TriggerHook registry (the P6 seam) proven with one real trigger, and a GET /api/events SSE endpoint with cursor resume for external subscribers. Emission points are instrumented at the existing choke points found in research: task create/update/delete queries (crates/db/src/models/task/queries.rs), execution-process lifecycle (ExecutionProcess::create as called by ContainerService::start_execution, and the completion path, in crates/services/src/services/container.rs), and hive connectivity transitions (HiveEvent::Connected/Disconnected in crates/services/src/services/hive_client.rs, plus the bulk-snapshot reconcile leg in crates/services/src/services/node_runner.rs). A compaction task bounds the journal.

## Design
Data model (crates/db, additive migration): event_journal table — seq INTEGER PRIMARY KEY AUTOINCREMENT, event_type TEXT NOT NULL, payload TEXT NOT NULL (JSON of the typed enum), created_at. Index on (event_type, seq). Directory module crates/db/src/models/event_journal/ (mod.rs, queries.rs: append within the model function's transaction, range-read by cursor, compact). Sibling to read before writing: crates/db/src/models/node_outbox.rs — note that node_outbox assigns seq by scalar subquery because its PK is id BLOB; event_journal's seq IS the primary key, so AUTOINCREMENT is the correct divergence and must be justified in the ledger.

Event schema (crates/db or crates/utils, TS-exported): enum NodeEvent with #[serde(tag = "type", rename_all = "snake_case")] — task_created, task_status_changed, task_deleted, attempt_started, attempt_finished, attempt_failed, hive_connected, hive_disconnected, reconcile_completed (fields: task_id/attempt_id/executor identity/old+new status/timestamps as applicable). Registered in generate_types.rs.

Bus core (crates/services/src/services/event_bus.rs): EventBus struct (Clone) wrapping tokio::sync::broadcast::Sender<SequencedEvent> where SequencedEvent = { seq, event: NodeEvent }. subscribe_from(cursor) hands off replay-to-live with this exact algorithm: (1) subscribe to the live broadcast channel FIRST; (2) capture the journal high-water mark (max seq); (3) replay journal rows (cursor, mark] in seq order; (4) drain the live receiver, discarding any buffered event with seq <= last-replayed (dedupe by seq monotonicity at the consumer edge); (5) on tokio broadcast Lagged(n), re-enter journal refill from the last-delivered seq before resuming live. This contract binds both consumers (SSE, TriggerHook runner) and is what TS2's skip/duplicate/lag tests assert.

Emission ownership (the transaction rule): the DB model function that performs the state write OWNS the transaction — it opens it, performs its existing discrete INSERT/UPDATE ... RETURNING statement, appends the event_journal row, commits, and only then publishes on the broadcast channel. Caller signatures are unchanged (they keep taking &SqlitePool), so no transaction is threaded through callers — this is why the node_outbox precedent's stated objection ("threading a shared txn through all Task::create callers is OUT of scope", crates/db/src/models/task/queries.rs:337) does not apply here. Sites: Task::create / Task::update / Task::update_status / Task::delete (crates/db/src/models/task/queries.rs) and ExecutionProcess::create (crates/db/src/models/execution_process/queries.rs:361, a single INSERT ... RETURNING; the git I/O that computes before_head_commit runs BEFORE it at crates/services/src/services/container.rs:1516-1523 and is therefore outside the transaction span) plus the execution-process completion write. Hive connectivity events have NO accompanying state write — the journal row IS the record — so they are appended directly with no transaction to share. Each site emits exactly one event per state change.

Bus/db layering: crates/db cannot depend on crates/services, so the broadcast::Sender<SequencedEvent> lives in the db layer (crates/db already depends on tokio — crates/db/Cargo.toml:20) and is held alongside the pool; crates/services/src/services/event_bus.rs wraps it to present EventBus and subscribe_from to service-layer and route-layer consumers. The journal append and the post-commit publish therefore both happen inside the db model function, preserving "never broadcast before commit" without an upward dependency.

Seq semantics: seq is monotonically increasing and never reused or regressed. It is NOT guaranteed contiguous — a rolled-back transaction may consume a seq value. Consumers MUST NOT infer a missed event from a hole in the integer sequence; the journal is the authority, and "no event skipped" means every row present in the journal above the cursor is delivered. Whether SQLite's AUTOINCREMENT actually leaks a value on rollback is asserted by a test rather than assumed.

Trigger hooks (the P6 seam): TriggerHook trait { fn matches(&self, event: &NodeEvent) -> bool; async fn fire(&self, event: SequencedEvent); } registered on the deployment at startup; hooks run on a dedicated task consuming subscribe_from(last_processed_seq) with per-hook cursor persisted in a small trigger_cursors table so hook processing survives restarts (at-least-once). Ship one real hook: a tracing/journal side-effect on task_status_changed used as the SC proof.

External endpoint: GET /api/events?cursor=N — SSE stream (existing SSE serving conventions); no cursor means live-only from now; with cursor replays journal then goes live; each SSE message carries seq so clients resume. The path is currently occupied by a consumer-less record-patch SSE route, which is removed FIRST as a standalone step: delete crates/server/src/routes/events.rs, unmount it from crates/server/src/routes/mod.rs (pub mod events; L20 and .merge(events::router(&deployment)) L72), and remove the now-orphaned Deployment::stream_events default trait method (crates/deployment/src/lib.rs:197-205, whose only caller is that route). EventService itself is NOT touched — it continues to back /api/tasks/stream/ws. The new route is then created at crates/server/src/routes/events.rs and mounted in routes/mod.rs base_routes under /api.

Retention/compaction: a periodic task (spawned like the existing WAL-monitor loop) deletes journal rows older than VK_EVENT_RETENTION_HOURS (default 168) while always retaining the newest VK_EVENT_MIN_ROWS (default 10000); both documented in .env.example. Compaction never deletes rows at or above the minimum persisted trigger cursor.

Offline/sync: no hive interaction anywhere; connectivity events are ABOUT the hive link, produced locally from the hive client's connection-state transitions.

## Decisions
D1 (irreversible — ADR dev-docs/adr/0017-durable-event-journal-bus.md): the bus contract is journal-first in-process broadcast with monotonic seq cursors, at-least-once delivery, and a single typed NodeEvent enum; external brokers, exactly-once, and hive relay are rejected. P6/P7 build on this contract.

D2 (reversible — clarified 2026-08-11): events are journaled in the same transaction as the discrete state-write STATEMENT (not the enclosing function), owned and committed by the DB model function, and broadcast only after commit — no phantom events for rolled-back changes, and no writer lock held across git/filesystem I/O. Connectivity events have no accompanying state write and are appended directly.

D3 (reversible): trigger hooks persist per-hook cursors (trigger_cursors table) and replay on restart — the P6 seam is crash-safe from day one.

D4 (reversible): the external surface is SSE (matching existing serving patterns) rather than WebSocket; WebSocket can be added later behind the same cursor semantics.

D5 (WITHDRAWN 2026-08-11): board live-update via react-query cache invalidation. Withdrawn because its premise was false — the board is not react-query-backed and does not poll (see Out of scope). No replacement; UI is out of scope this phase.

D6 (reversible): retention defaults (VK_EVENT_RETENTION_HOURS=168, VK_EVENT_MIN_ROWS=10000) are env-tunable; compaction never crosses the minimum persisted trigger cursor.

D7 (reversible, added 2026-08-11): the bus reuses the /api/events path; the pre-existing consumer-less record-patch SSE route and its orphaned Deployment::stream_events trait method are deleted first, as a standalone step ahead of any bus code. Chosen over a new path (e.g. /api/node-events) so the public contract P7 and external tools bind to is unambiguous, and so dead code is removed while it is still cheap.

D8 (reversible, added 2026-08-11): the broadcast sender lives in crates/db and crates/services/src/services/event_bus.rs wraps it, because crates/db cannot depend on crates/services and the post-commit publish must happen where the transaction commits.

D9 (reversible, added 2026-08-11): seq is monotonic but not contiguous; consumers must not treat a seq hole as a missed event. Accepted as the cost of assigning seq inside a transaction that may roll back.

## Test strategy
TS1: DB layer: sqlx tests via db::test_utils::create_test_pool() for journal append-in-transaction (rollback journals nothing), cursor range reads, seq monotonicity across a rolled-back transaction, and compaction respecting both retention floor and minimum trigger cursor.
TS2: Bus core: unit tests for post-commit-only broadcast, subscribe_from catch-up-then-live chaining with no journaled event skipped and tolerated duplicates, and slow-consumer lag handling on the broadcast channel.
TS3: Emission: integration tests asserting exactly one correctly-typed event per instrumented state change across task CRUD, attempt start/finish/fail, and hive connect/disconnect/reconcile transitions.
TS4: Trigger hooks: tests for cursor persistence and replay-after-restart semantics (at-least-once, no loss) using a recording test hook.
TS5: API: route tests for /api/events SSE (cursor resume, seq framing), plus a guard asserting the removed record-patch route and Deployment::stream_events are gone.
TS6: Live acceptance on a deployed node per the SC list, including kill/restart durability (SC5) and offline coverage (SC8); evidence recorded in the decisions-ledger and verify_cmd green post-deploy.

## Amendment history
2026-08-11 — amended during /wai:decompose after the breakdown found four spec-vs-reality collisions against merged main. The original spec was frozen at spec_sha=c9a1dc75de0760d4160333b3eb84cdd1515aae12; /wai:precheck had been re-run with --no-anchor-check for a genuine truncated-prefix false positive, which is why the anchor collisions were not caught earlier. Changes, all authorised by the spec owner: (1) GET /api/events was already taken by a consumer-less record-patch SSE route — repurpose the path, deleting the old route first (D7, Design "External endpoint"). (2) The board has no poll loop and is not react-query-backed — UI live-update dropped entirely, SC7 deleted, D5 withdrawn, US1 restated to the durable-observability outcome this phase delivers, TS5/TS6 narrowed (Out of scope). (3) HiveSyncService has no connect/disconnect/reconcile transitions — connectivity anchors repointed to hive_client.rs and node_runner.rs (Approach, Design). (4) No emission site had a caller transaction, and wrapping start_execution would have held the SQLite writer lock across git I/O — resolved by having the DB model function own a transaction around its own discrete write statement, which keeps caller signatures unchanged (D2, Design "Emission ownership"). Two gaps the collisions exposed were also closed: the crates/db → crates/services layering of the broadcast sender (D8) and the non-contiguous seq contract (D9). SC/US/TS ids were NOT renumbered per schema/spec.frontmatter.md — SC7 is deleted and its number left vacant.
