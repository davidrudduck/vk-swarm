# ADR-0017: Node event bus = in-process broadcast over a durable, cursor-replayable journal

- **Status:** accepted (amended twice on 2026-08-11 — see Amendment and Amendment 2 below)
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
  transaction as the **discrete state-write statement** they describe, appended by the DB model
  function performing that write. (Amendment 2: the transaction may be opened by that model
  function OR owned by its caller — `append` is generic over sqlx `Executor` — and the model
  function does not publish.) Where a lifecycle event has no accompanying
  state write (hive connectivity transitions), the journal row IS the record and is appended
  directly.
- An in-process `tokio::sync::broadcast` bus fans out live events after commit. (Amendment 2:
  "after commit" is now enforced structurally by a journal tailer rather than by the model function
  publishing; the `Sender` lives in `crates/services`, not `crates/db`.)
- Consumers (internal trigger hooks and the external SSE endpoint) resume from any
  `seq` cursor by reading the journal, then switch to live — **at-least-once, no retained
  journaled event skipped, duplicates possible; consumers must be idempotent**. Compaction
  (Amendment 7) bounds retained history: a cursor below the journal low-water mark replays
  only retained rows. Persisted trigger cursors get an explicit `needs_rebootstrap` signal;
  SSE clients must treat a cursor older than retained history as a full-refresh case (the
  stream does not report the compacted range).
- Replay-to-live handoff contract (all consumers): subscribe to the live channel FIRST,
  capture the journal high-water mark, replay journal rows `(cursor, mark]` in seq order,
  drain buffered live events discarding `seq <= last-replayed`, and on broadcast `Lagged(n)`
  refill from the journal at the last-delivered seq before resuming live. Lag never skips a
  retained journaled event — it degrades to a journal re-read.
- `seq` is monotonic over committed, observable rows and never reused or regressed among
  them, but **not contiguous** as a matter of contract: consumers must tolerate holes. A hole
  in the integer sequence is not evidence of a missed event; the journal is the authority.
  (Amendment 2 corrected the original rationale here: SQLite reverts and reuses a rolled-back
  allocation, which is never observable — see Amendment 8 for the committed-row scope.)
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

## Amendment 2 — 2026-08-11 (after the adversarial breakdown review)

Amended in place again, still before any implementation existed, following tournament 1 of
`/wai:decompose`. Two cross-model competitors independently found that the emission mechanism
described in Amendment 1 could not be built as written; the finding was peer-validated and
verified against the repo before the spec owner decided.

5. **Publication is decoupled from emission by a journal tailer.** Amendment 1 said the DB model
   function commits its transaction "and only then publishes on the broadcast channel". That is
   unbuildable: the sender was to live in the db layer held alongside the pool, but model functions
   receive only `&SqlitePool`, and a pool has no back-reference to the `DBService` that owns it.
   The repairs both cost more than they save — a process-global `OnceLock` in `crates/db` would
   capture the *bootstrap* sender, because production constructs `DBService::bootstrap()` before the
   live service at `crates/local-deployment/src/lib.rs:155-166`, and would additionally cross-publish
   between in-process test pools; moving the public API onto `DBService` emitting wrappers is
   correct but forces migration of every caller of the six emission functions.

   Instead, model functions **append only**, and a per-`DBService` background task tails the journal
   and publishes rows in `seq` order. Since a row is only readable once its transaction has
   committed, **journal-first/broadcast-second becomes structural** rather than a convention an
   implementer can forget, and a rolled-back transaction cannot produce a phantom broadcast. The
   broadcast `Sender` therefore moves UP to `crates/services`, and `crates/db` needs no sender at
   all — which is what makes "caller signatures stay `&SqlitePool`" satisfiable.

   Accepted cost: publication latency is bounded by the tail interval rather than immediate. Every
   named consumer (P6 triggers, P7 MCP/ACP observability, the SSE endpoint) is non-interactive, so
   this is immaterial. Reversible: restoring synchronous publish later needs only a sender path at
   the emission sites, and changes none of the seq, cursor, or at-least-once contracts P6/P7 bind to.

6. **A generic append dissolves the delete problem.** `EventJournal::append` is generic over sqlx
   `Executor`, so it composes both with a model function that opens its own transaction and with one
   handed a caller-owned transaction. This matters because `Task::delete` is already generic over
   `E: Executor` and its local route (`crates/server/src/routes/tasks/handlers/core.rs:655-670`)
   owns an outer transaction spanning child nullification — so "the model owns the transaction"
   could never have applied there.

7. **A hard cap bounds the journal.** "Compaction never deletes at or above the minimum persisted
   trigger cursor" cannot coexist with "the journal cannot grow unbounded" when a hook's cursor stops
   advancing. `VK_EVENT_MAX_ROWS` now overrides the cursor floor and marks the passed cursors as
   requiring rebootstrap, so a dead consumer degrades to explicit, observable event loss instead of
   unbounded growth.

8. **The `seq` non-contiguity rationale was factually wrong.** Amendment 1 (point 3) said assigning
   `seq` inside a transaction that may roll back "can leak a value". A direct probe shows SQLite
   **reuses** it — `sqlite_sequence` is itself transactional, so the rollback reverts the allocation.
   The consumer-facing contract is unchanged and remains correct, because it is the conservative
   direction: consumers must tolerate holes regardless of whether this mechanism creates them.
   To be precise about scope: the no-reuse guarantee applies to **committed, observable rows** —
   a rolled-back allocation is never observable (no consumer, cursor, or dedup key can have seen
   it), so SQLite reusing that value cannot violate any cursor or deduplication contract. Only
   the stated cause was wrong, and the behaviour is now asserted by a test rather than assumed.

The irreversible core is still unchanged. What moved is the publication mechanism, which was never
part of it.
