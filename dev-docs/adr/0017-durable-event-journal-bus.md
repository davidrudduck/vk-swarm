# ADR-0017: Node event bus = in-process broadcast over a durable, cursor-replayable journal

- **Status:** accepted (amended 2026-08-11 — see Amendment below)
- **Date:** 2026-08-07
- **Workstream:** vk-swarm-event-bus
- **Spec:** `docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md`

## Context

P4 (SC4) needs task/attempt/connectivity lifecycle events that downstream triggers consume —
P6's management agent, P7's MCP/ACP observability, and UI live-update. Candidate transports:
an external broker (NATS/Redis), hive-relayed events, or a node-internal bus. An external
broker adds operational surface and violates offline-first; hive relay makes events die with
connectivity. The node already proves the needed durability pattern: the ordered ack'd
`node_outbox` (`crates/db/migrations/20260201000400_add_node_outbox.sql`) and
SQLite-as-node-of-record.

> Note (2026-08-11): the UI-live-update motivation recorded above was found to be already
> satisfied by shipped code and has been dropped from the spec. See Amendment.

## Decision

Events are **journaled first, broadcast second**, entirely node-local:

- New `event_journal` table: monotonic `seq` (INTEGER PRIMARY KEY AUTOINCREMENT),
  `event_type`, typed JSON `payload`, `created_at`. Journal writes happen in the same
  transaction as the **discrete state-write statement** they describe, owned and committed by
  the DB model function performing that write. Where a lifecycle event has no accompanying
  state write (hive connectivity transitions), the journal row IS the record and is appended
  directly.
- An in-process `tokio::sync::broadcast` bus fans out live events after commit.
- Consumers (internal trigger hooks and the external SSE endpoint) resume from any
  `seq` cursor by reading the journal, then switch to live — **at-least-once, no journaled
  event skipped, duplicates possible; consumers must be idempotent**.
- Replay-to-live handoff contract (all consumers): subscribe to the live channel FIRST,
  capture the journal high-water mark, replay journal rows `(cursor, mark]` in seq order,
  drain buffered live events discarding `seq <= last-replayed`, and on broadcast `Lagged(n)`
  refill from the journal at the last-delivered seq before resuming live. Lag never skips a
  journaled event — it degrades to a journal re-read.
- `seq` is monotonic and never reused or regressed, but **not contiguous**: a rolled-back
  transaction may consume a value. A hole in the integer sequence is not evidence of a missed
  event; the journal is the authority.
- The event schema is one Rust enum (snake_case serde, ts-rs export) — the single typed
  contract for backend and external subscribers.
- Retention: journal is bounded by a compaction policy (age + row-count floor, env-tunable)
  so long-lived nodes cannot grow unbounded.

Explicitly rejected: external brokers (revisit only if multi-host fan-out beyond the hive
demands it), exactly-once delivery (idempotence is cheaper than distributed transactions),
and hive-side relay (nodes already sync state; events are a node-local concern this phase).

## Consequences

- Positive: no new infrastructure; offline-first preserved; crash durability and replay come
  from patterns P2 already proved; the seq/cursor contract gives P6/P7 a stable substrate.
- Negative / accepted cost: fan-out is single-node only; cross-node aggregation is a later
  hive concern. Journal writes add one insert per lifecycle change (bounded by compaction).
  `seq` may contain holes, so consumers must not equate a hole with data loss.
- Irreversibility: the **seq/cursor + at-least-once + typed-enum contract** is what P6
  triggers and P7 connectivity will build against; walking it back after those phases start
  means rewriting their foundation. Hence this ADR.

## Amendment — 2026-08-11

Amended in place (not superseded) during `/wai:decompose`, before any implementation existed.
The ADR was four days old, unreleased, and no code depended on it; superseding a decision
nothing had consumed would have added indirection without adding information. `dev-docs/adr/`
had no prior amendment or superseding convention, so this establishes amend-in-place with a
dated section.

What changed and why:

1. **The transaction rule was clarified, not weakened.** The original text — "the same
   transaction as the state change they describe wherever one exists" — turned out to describe
   no existing call site: `Task::create`/`update` (`crates/db/src/models/task/queries.rs:290`,
   `:327`) and `ExecutionProcess::create`
   (`crates/db/src/models/execution_process/queries.rs:361`) all take `pool: &SqlitePool` and
   run outside any transaction, and the `node_outbox` precedent explicitly declined transaction
   threading as out of scope (`queries.rs:337`). The resolution is that the **DB model function
   owns the transaction around its own discrete write statement**, leaving caller signatures
   unchanged — so no transaction is threaded through callers and the precedent's stated
   objection does not apply. This deliberately excludes wrapping enclosing service functions:
   `ContainerService::start_execution` performs git I/O at
   `crates/services/src/services/container.rs:1516-1523`, and holding SQLite's single writer
   lock across it would be a node-wide liveness hazard.
2. **Connectivity events are journal-only.** `HiveEvent::Connected`/`Disconnected`
   (`crates/services/src/services/hive_client.rs:907`, `:819`) perform no DB write, so there is
   no transaction to share and no consistency window to protect.
3. **`seq` non-contiguity was made explicit.** Assigning `seq` inside a transaction that may
   roll back can leak a value. "Gap-free" was replaced throughout with "no journaled event
   skipped" so consumers cannot misread a numbering hole as loss.
4. **The UI was removed as a consumer.** The board already streams live over
   `/api/tasks/stream/ws` and does not poll (`frontend/src/hooks/useProjectTasks.ts:80-92`).
   A domain event log is the wrong transport for view state; the read-model push channel stays.
   The Context section above is left as originally written — it records what was believed on
   2026-08-07 — with a pointer to this amendment.

The irreversible core of the ADR — journal-first, in-process broadcast, monotonic seq cursors,
at-least-once delivery, one typed enum, no external broker — is unchanged.
