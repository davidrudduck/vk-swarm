# ADR-0008 — Node→hive sync via a single ordered, acknowledged outbox / op-log

- **Status:** accepted
- **Date:** 2026-06-30
- **Workstream:** vk-swarm-hive-redesign
- **Supersedes behaviour of:** the five independent node→hive push paths in `hive_sync.rs` +
  `share/publisher.rs`

## Context

Verified in code (file:line):

- The node has **five independent outbound push paths** — task link (WS `TaskSync`,
  `hive_sync.rs:303`), attempt (WS `AttemptSync`, `:368`), execution process (WS `ExecutionSync`,
  `:434`), logs (WS `LogsBatch`, `:562`), and the share-task **HTTP POST**
  (`share/publisher.rs:109`). Four ride one `mpsc::channel(64)` outbox (`hive_client.rs:723`).
- **Cross-entity ordering is already enforced declaratively** — attempt sync skips when the parent
  task is unlinked (`hive_sync.rs:340-349`, the load-bearing `continue`); exec and log `find_unsynced`
  carry SQL JOIN guards on the parent's `hive_synced_at` (`execution_process/sync.rs:22-23`,
  `log_entry/sync.rs:26-27`). So ordering is *guardable*, not the defect.
- **The defect is ack timing.** For attempts/execs/logs the dirty flag is cleared by
  `mark_hive_synced_batch` **immediately after the mpsc enqueue** (`hive_sync.rs:380-382,446-448,574-577`),
  while the socket write happens later with **no ack** (`hive_client.rs:890-895`). A crash or drop
  between enqueue and durable hive receipt **permanently marks the entity synced** and it is never
  retried (`find_unsynced` filters it out) → silent write loss (§2.6, §2.7).
- **No monotonic seq/version on child entities** — only `TaskSyncMessage` carries a hardcoded
  `version: 1` (`hive_sync.rs:289`); `AttemptSyncMessage`/`ExecutionSyncMessage`/`LogsBatchMessage`
  carry none, and no `*SyncResponse` ack types exist.

## Decision

Replace the five push paths with **one per-node append-only op-log** in node SQLite:

```text
outbox(seq INTEGER PK AUTOINCREMENT,   -- per-node monotonic
       op_type, entity_type, entity_id,
       payload, idempotency_key UNIQUE, fencing_token,
       created_at, acked_at NULL)
```

- **Ordered, single channel (SC2a).** Ops are appended in causal order; the existing
  task→attempt→exec→log dependency becomes explicit op ordering. The node streams ops in `seq` order
  over the WS — one channel, FIFO.
- **Parent-before-child by construction (SC2b).** The hive **parks** a child op whose parent op has not
  yet been applied and applies it once the parent lands; the node never emits a child before its
  parent op.
- **Acknowledged, no silent loss (SC2c).** The hive applies each op **idempotently**
  (`idempotency_key` UNIQUE, `ON CONFLICT DO NOTHING`) and returns a **durable ack** carrying the
  applied high-water `seq`. The node sets `acked_at` and advances its cursor **only on durable ack** —
  the flag-clear-before-ack window is closed.
- **Per-op monotonic version** replaces the hardcoded `version: 1`.
- **Anti-entropy reconciliation (SC5)** rides this contract: on reconnect (and periodically) the node
  re-streams from its unacked cursor, and a **digest exchange** over the frozen `Digest`/`DigestEntry`
  contract shape (CONTRACT §A) detects silent divergence the cursor alone would miss. The
  `DigestEntry` carries `entity_type`/`entity_id`/`version` per swarm-linked task; the per-entity
  version/hash + outbox high-water are **local heal inputs** the node computes to drive its own
  re-stream/pull decisions — they are NOT protocol payload fields beyond the frozen `DigestEntry`
  shape, and they do NOT widen the contract. The hive replies with the ops/pulls needed to heal. This
  *is* the replacement for the manual `reset_*` repair migrations — the repurposed bulk-snapshot
  reconcile ([ADR-0007](./0007-single-inbound-channel-one-delete-one-conflict.md)) is its gap-fill leg.
- An op **against a hive-assigned task** carries that task's current **fencing token**
  ([ADR-0009](./0009-lease-checkout-fencing.md)) so stale-lease writes are rejected at apply.
  Node-owned work (locally-created tasks + their attempts/execs/logs — the majority of ops) carries
  **no token** and commits under the node's ownership identity, not a lease.

## Consequences

- Closes the silent-write-loss window for attempts/execs/logs (SC2c) and guarantees cross-entity
  ordering by construction (SC2b), not best-effort.
- Deletes the four WS push paths and the HTTP share-publisher path as the sync mechanism (the share/
  link operation becomes the task op at the head of the log). Irreversible wire-format + code deletion.
- The op-log is the single event source later phases (P3/P6) consume.

## Alternatives considered

- **Add acks to the existing five paths** (targeted bracket, analysis §2.6) — rejected: keeps five
  parallel cursors, no single ordering guarantee, and the share path stays a separate transport. The
  rebuild was decided at the program level (analysis §2.6); this ADR records the op-log realization.
- **Vector clocks / CRDT merge** — rejected: the hub-and-spoke topology makes the hive authoritative,
  so a per-node monotonic op-log with idempotency keys is sufficient and far simpler.
