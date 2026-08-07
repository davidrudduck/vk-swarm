---
doc_type: spec
status: draft
workstream: vk-swarm-event-bus
change_kind: behaviour
---

# vk-swarm-event-bus — task-lifecycle event bus (P4 / SC4)

Phase 4 of the `vk-swarm-refactor` program
(`docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md`). Owns umbrella success criterion
**SC4**: "Task lifecycle changes emit events on an internal/external bus that downstream
triggers consume." Independent of P3 (P3 ⟂ P4); together they gate P5–P7.

## Intent (what / why)

Give every node a durable, subscribable event bus so that things *happening* in vk-swarm
(task changes, attempt/executor runs, hive connectivity) become first-class, ordered,
replayable events instead of state that consumers must poll for. This is the nervous system
the later phases attach to: P6's management agent reacts to bus triggers, P7 exposes bus
observability over MCP/ACP, and the UIs stop polling.

Decisions settled at intent time (interview 2026-08-07):

- **Transport: in-process bus + durable outbox bridge.** A tokio broadcast bus inside the
  node process, with events journaled through the same durable-outbox pattern P2 proved for
  hive sync, so events survive crashes and restarts. No external broker; offline-first is
  preserved. External consumers reach the bus via a node API subscription endpoint
  (SSE/WebSocket).
- **Event sources at ship (three):** (1) task lifecycle — created / status-changed / deleted,
  including P3's proposal-created / proposal-accepted once P3 lands; (2) attempt & execution
  lifecycle — attempt started / finished / failed and executor process events (what P6 most
  needs); (3) sync/connectivity — hive connected / disconnected, reconcile completed. Git/PR
  events are explicitly deferred.
- **Day-one consumers (three, each proven live):** (1) an internal trigger-hook subscription
  API — the seam P6 plugs into — demonstrated with at least one real working trigger; (2) an
  external SSE/WebSocket subscription endpoint (paves P7 observability); (3) UI live-update —
  the node UI consumes the bus for board refresh instead of polling.
- **Delivery guarantee: at-least-once with replay.** Events are durably journaled with
  monotonic sequence numbers; a consumer reconnecting with a cursor replays everything it
  missed. Consumers must tolerate duplicates (idempotent handling). Matches the ordered
  ack'd-outbox semantics already proven in the P2 hive sync.

## Users / who is affected

- **Operators**: live board updates without refresh/poll lag; ops visibility into
  connectivity and reconcile events.
- **P6 management agent (future)**: consumes trigger hooks to select ready work — the primary
  design customer for ordering/replay semantics.
- **External tools / P7 fabric (future)**: subscribe over the external endpoint to observe
  runs without touching internal state.
- **Frontend/UI code**: migrates from polling hooks to bus-driven updates where the bus
  covers the data.

## Success criteria

Runtime-observable on a running node (not "test X passes"):

- **SC-A (emission).** Creating, moving (status change), and deleting a task on a running
  node each produce a journaled event with a monotonic sequence number, observable via the
  subscription endpoint and queryable after the fact.
- **SC-B (attempt coverage).** Starting a task attempt and its terminal outcome (finished /
  failed) each emit events carrying task id, attempt id, and executor identity.
- **SC-C (connectivity coverage).** Killing and restoring the hive connection on a running
  node emits disconnected / connected / reconcile-completed events in order.
- **SC-D (external subscription + replay).** An external client (curl/websocat) subscribing
  with no cursor receives live events; disconnecting, causing N events, and resubscribing
  with its last-seen cursor delivers exactly the missed events (possibly with duplicates,
  never gaps), in sequence order.
- **SC-E (crash durability).** Events journaled before a node process kill are still
  replayable after restart; sequence numbering continues without reuse or regression.
- **SC-F (internal trigger).** A registered trigger hook demonstrably fires on a matching
  event on the live node (e.g. a log/journal side-effect on task-status-change) — the P6
  seam proven end-to-end.
- **SC-G (UI live-update).** With two browser windows on the same node board, a task moved in
  one appears moved in the other without manual refresh, driven by the bus subscription (not
  a poll loop) — verifiable via the network tab.
- **SC-H (offline-first).** SC-A/-B/-D/-F all hold with the hive unreachable.

## Constraints

- **Reuse P2 patterns, don't invent:** the durable ordered ack'd outbox, local
  SQLite-as-journal, existing SSE/WebSocket serving patterns in the server crate,
  `ApiResponse<T>` routes, ts-rs typegen for event payload types.
- **Offline-first non-negotiable:** the bus is fully functional with no hive; hive relay of
  events is out of scope here (nodes already sync state up — do not duplicate that channel).
- **Bounded journal:** the event journal must have a retention/compaction policy (defined at
  `/wai:spec`) so it cannot grow unbounded on long-lived nodes.
- **Typed events:** event schema is a Rust enum serialized snake_case, exported to TypeScript
  via ts-rs — one source of truth; no stringly-typed event names in the frontend.
- **No breaking of existing streams:** current SSE/log-streaming endpoints keep working;
  UI migration to the bus is per-surface, not big-bang.
- **Reference systems read-only:** paperclip for trigger/governance prior art.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **External message brokers** (NATS/Redis/Kafka) — revisit only if multi-host fan-out
  beyond the hive ever demands it.
- **Git/PR events** (branch pushed, PR opened/merged) — deferred to a later increment.
- **Hive-side bus or cross-node event aggregation** — the hive keeps consuming state sync;
  aggregating node buses is a later-phase concern.
- **Trigger *logic*** beyond one proof-of-seam hook — rule engines, priority selection, and
  automation policy belong to P5/P6.
- **Exactly-once delivery** — consumers are required to be idempotent instead.
- **Event-driven rewrite of every UI surface** — only the proof surface (board live-update)
  ships here.
