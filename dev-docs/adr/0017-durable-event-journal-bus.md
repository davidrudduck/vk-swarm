# ADR-0017: Node event bus = in-process broadcast over a durable, cursor-replayable journal

- **Status:** accepted
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

## Decision

Events are **journaled first, broadcast second**, entirely node-local:

- New `event_journal` table: monotonic `seq` (INTEGER PRIMARY KEY AUTOINCREMENT),
  `event_type`, typed JSON `payload`, `created_at`. Journal writes happen in the same
  transaction as the state change they describe wherever one exists.
- An in-process `tokio::sync::broadcast` bus fans out live events after commit.
- Consumers (internal trigger hooks, the external SSE endpoint, the UI) resume from any
  `seq` cursor by reading the journal, then switch to live — **at-least-once, gap-free,
  duplicates possible; consumers must be idempotent**.
- The event schema is one Rust enum (snake_case serde, ts-rs export) — the single typed
  contract for backend, frontend, and external subscribers.
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
- Irreversibility: the **seq/cursor + at-least-once + typed-enum contract** is what P6
  triggers, P7 connectivity, and UI subscriptions will build against; walking it back after
  those phases start means rewriting their foundation. Hence this ADR.
