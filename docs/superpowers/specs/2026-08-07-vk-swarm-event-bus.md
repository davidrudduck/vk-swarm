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

Give every node a durable, subscribable event bus so that things happening in vk-swarm (task changes, attempt/executor runs, hive connectivity) become first-class, ordered, replayable events instead of state that consumers must poll for. This is the nervous system the later phases attach to: P6's management agent reacts to bus triggers, P7 exposes bus observability over MCP/ACP, and the UIs stop polling.

Decisions settled at intent time (interview 2026-08-07): transport is an in-process bus bridged to a durable journal (no external broker); ship-scope event sources are task lifecycle, attempt/execution lifecycle, and sync/connectivity (git/PR events deferred); day-one consumers are an internal trigger-hook API (the P6 seam, proven with one real trigger), an external SSE subscription endpoint, and UI live-update for the board; delivery is at-least-once with cursor replay — consumers must be idempotent.


## User stories
- **US1:** As the operator, board changes made anywhere on the node appear live in my open board views without manual refresh or polling.
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
SC4: An external client subscribing with no cursor receives live events; disconnecting, causing N events, and resubscribing with its last-seen cursor delivers all missed events (duplicates allowed, gaps never) in seq order.
→ US2
SC5: Events journaled before a node process kill are still replayable after restart; seq numbering continues without reuse or regression.
→ US2
SC6: A registered trigger hook demonstrably fires on a matching event on the live node (observable side-effect on task_status_changed), and after a node restart it resumes from its persisted cursor without losing the events in between.
→ US3
SC7: With two browser windows on the same node board, a task moved in one appears moved in the other without manual refresh, driven by the event subscription (verifiable in the network tab: no task-list poll loop on the board surface).
→ US1
SC8: SC1, SC2, SC4, and SC6 all hold with the hive unreachable.
→ US2

## Users
Operators: live board updates without refresh/poll lag; ops visibility into connectivity and reconcile events.

P6 management agent (future): consumes trigger hooks to select ready work — the primary design customer for ordering/replay semantics.

External tools / P7 fabric (future): subscribe over the external endpoint to observe runs without touching internal state.

Frontend/UI code: migrates from polling hooks to bus-driven updates where the bus covers the data.


## Constraints
Reuse P2 patterns, do not invent: the durable ordered ack'd outbox precedent (crates/db/migrations/20260201000400_add_node_outbox.sql), local SQLite-as-journal, existing SSE/streaming serving patterns in the server crate (container.rs stream_raw_logs/stream_normalized_logs precedent), ApiResponse<T> routes, ts-rs typegen with manual registration in crates/server/src/bin/generate_types.rs.

Offline-first non-negotiable: the bus is fully functional with no hive; hive relay of events is out of scope (nodes already sync state up — do not duplicate that channel).

Bounded journal: the event journal must have a retention/compaction policy so it cannot grow unbounded on long-lived nodes (env-tunable per the VK_* convention in .env.example).

Typed events: the event schema is a Rust enum serialized snake_case, exported to TypeScript via ts-rs — one source of truth; no stringly-typed event names in the frontend.

No breaking of existing streams: current SSE/log-streaming endpoints keep working; UI migration to the bus is per-surface, not big-bang.

Reference systems read-only: paperclip for trigger/governance prior art.

GitHub targeting: PRs only against davidrudduck/vk-swarm.


## Out of scope
External message brokers (NATS/Redis/Kafka) — revisit only if multi-host fan-out beyond the hive ever demands it (ADR-0017 records the rejection).

Git/PR events (branch pushed, PR opened/merged) — deferred to a later increment.

Hive-side bus or cross-node event aggregation — the hive keeps consuming state sync; aggregating node buses is a later-phase concern.

Trigger logic beyond one proof-of-seam hook — rule engines, priority selection, and automation policy belong to P5/P6.

Exactly-once delivery — consumers are required to be idempotent instead.

Event-driven rewrite of every UI surface — only the proof surface (board live-update) ships here.


## Approach
Journal first, broadcast second (ADR-0017). Every covered lifecycle change writes a typed event row into a new event_journal table — in the same SQLite transaction as the state change wherever one exists — then publishes the committed event on an in-process tokio broadcast channel. Consumers get gap-free at-least-once delivery by reading the journal from a seq cursor and then switching to the live channel. Three consumer surfaces ship: an internal TriggerHook registry (the P6 seam) proven with one real trigger, a GET /api/events SSE endpoint with cursor resume for external subscribers, and a frontend hook that drives board live-update by invalidating react-query caches on task events. Emission points are instrumented at the existing choke points found in research: task create/update/delete queries (crates/db/src/models/task/queries.rs), execution-process lifecycle (ContainerService start_execution / completion handling in crates/services/src/services/container.rs), and HiveSyncService connect/disconnect/reconcile (crates/services/src/services/hive_sync.rs). A compaction task bounds the journal.


## Design
Data model (crates/db, additive migration): event_journal table — seq INTEGER PRIMARY KEY AUTOINCREMENT, event_type TEXT NOT NULL, payload TEXT NOT NULL (JSON of the typed enum), created_at. Index on (event_type, seq). Directory module crates/db/src/models/event_journal/ (mod.rs, queries.rs: append within caller's transaction, range-read by cursor, compact).

Event schema (crates/db or crates/utils, TS-exported): enum NodeEvent with #[serde(tag = "type", rename_all = "snake_case")] — task_created, task_status_changed, task_deleted, attempt_started, attempt_finished, attempt_failed, hive_connected, hive_disconnected, reconcile_completed (fields: task_id/attempt_id/executor identity/old+new status/timestamps as applicable). Registered in generate_types.rs; frontend consumes the generated union type.

Bus core (crates/services/src/services/event_bus.rs): EventBus struct (Clone) holding tokio::sync::broadcast::Sender<SequencedEvent> where SequencedEvent = { seq, event: NodeEvent }. emit(&mut tx, event) appends to event_journal inside the caller's transaction and stages the broadcast; a post-commit hook publishes staged events (never broadcast before commit). subscribe_from(cursor) hands off replay-to-live with this exact algorithm: (1) subscribe to the live broadcast channel FIRST; (2) capture the journal high-water mark (max seq); (3) replay journal rows (cursor, mark] in seq order; (4) drain the live receiver, discarding any buffered event with seq <= last-replayed (dedupe by seq monotonicity at the consumer edge); (5) on tokio broadcast Lagged(n), re-enter journal refill from the last-delivered seq before resuming live. This contract binds all three consumers (SSE, UI hook, TriggerHook runner) and is what TS2's gap/duplicate/lag tests assert.

Emission instrumentation: Task::create/update/delete (queries.rs — alongside the existing enqueue_task_upsert_op outbox calls), ContainerService::start_execution + the completion path that consumes next_action (container.rs), HiveSyncService connect/disconnect/reconcile transitions (hive_sync.rs). Each site emits exactly one event per state change.

Trigger hooks (the P6 seam): TriggerHook trait { fn matches(&self, event: &NodeEvent) -> bool; async fn fire(&self, event: SequencedEvent); } registered on the deployment at startup; hooks run on a dedicated task consuming subscribe_from(last_processed_seq) with per-hook cursor persisted in a small trigger_cursors table so hook processing survives restarts (at-least-once). Ship one real hook: a tracing/journal side-effect on task_status_changed used as the SC proof.

External endpoint (crates/server/src/routes/events.rs): GET /api/events?cursor=N — SSE stream (existing SSE serving conventions); no cursor means live-only from now; with cursor replays journal then goes live; each SSE message carries seq so clients resume. Mounted in routes/mod.rs base_routes under /api.

UI live-update (frontend/src): useNodeEvents hook (EventSource on /api/events with last-seq resume) mounted at board level; on task_* events it invalidates ['tasks', projectId] react-query keys (replacing the poll loop for the board surface only). Generated NodeEvent types from shared/types.ts; no string literals.

Retention/compaction: a periodic task (spawned like the existing WAL-monitor loop) deletes journal rows older than VK_EVENT_RETENTION_HOURS (default 168) while always retaining the newest VK_EVENT_MIN_ROWS (default 10000); both documented in .env.example. Compaction never deletes rows at or above the minimum persisted trigger cursor.

Offline/sync: no hive interaction anywhere; connectivity events are ABOUT the hive link, produced locally by HiveSyncService state transitions.


## Decisions
D1 (irreversible — ADR dev-docs/adr/0017-durable-event-journal-bus.md): the bus contract is journal-first in-process broadcast with monotonic seq cursors, at-least-once delivery, and a single typed NodeEvent enum; external brokers, exactly-once, and hive relay are rejected. P6/P7/UI build on this contract.

D2 (reversible): events are journaled in the same transaction as the state change they describe and broadcast only after commit — no phantom events for rolled-back changes.

D3 (reversible): trigger hooks persist per-hook cursors (trigger_cursors table) and replay on restart — the P6 seam is crash-safe from day one.

D4 (reversible): the external surface is SSE (matching existing serving patterns) rather than WebSocket; WebSocket can be added later behind the same cursor semantics.

D5 (reversible): board live-update is implemented as react-query cache invalidation driven by events, not a bespoke state channel — smallest change that removes the poll loop for the proof surface.

D6 (reversible): retention defaults (VK_EVENT_RETENTION_HOURS=168, VK_EVENT_MIN_ROWS=10000) are env-tunable; compaction never crosses the minimum persisted trigger cursor.


## Test strategy
TS1: DB layer: sqlx tests via db::test_utils::create_test_pool() for journal append-in-transaction (rollback emits nothing), cursor range reads, seq monotonicity, and compaction respecting both retention floor and minimum trigger cursor.
TS2: Bus core: unit tests for post-commit-only broadcast, subscribe_from catch-up-then-live chaining with no gaps and tolerated duplicates, and slow-consumer lag handling on the broadcast channel.
TS3: Emission: integration tests asserting exactly one correctly-typed event per instrumented state change across task CRUD, attempt start/finish/fail, and hive connect/disconnect/reconcile transitions.
TS4: Trigger hooks: tests for cursor persistence and replay-after-restart semantics (at-least-once, no loss) using a recording test hook.
TS5: API + frontend: route tests for /api/events SSE (cursor resume, seq framing); vitest for useNodeEvents (resume, invalidation on task events) and for absence of the board poll loop on the migrated surface.
TS6: Live acceptance on a deployed node per the SC list, including the two-window live-update check (SC7), kill/restart durability (SC5), offline coverage (SC8); evidence recorded in the decisions-ledger and verify_cmd green post-deploy.

