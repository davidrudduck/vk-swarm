---
id: "101"
phase: 1
title: Add node_outbox table migration (SQLite)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/db/migrations/20260201000400_add_node_outbox.sql
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: []
---
## Failing test (write first)
N/A — covered by task 104's `node_outbox` round-trip test (`OutboxRepository` enqueue/peek/ack), which
runs against this migration via `db::test_utils::create_test_pool()` (the template DB applies ALL
migrations, so a malformed migration fails every `db` test build). This task is a pure additive schema
migration with no Rust code; its correctness is proven when 104's persistence test goes green.

## Change
- **File:** `crates/db/migrations/20260201000400_add_node_outbox.sql` (NEW)
- **Anchor:** new migration file; timestamp prefix must sort AFTER the latest existing SQLite migration
  (`20260201000300_add_fence_attempt_count.sql`). Use `20260201000400` (or a later UTC stamp if a newer
  migration has landed — STOP-check below).
- **Sibling read (rubric #9):** modeled on `crates/db/migrations/20260201000000_add_queued_messages.sql`
  — same conventions: `CREATE TABLE IF NOT EXISTS`, BLOB UUID PKs, `TEXT NOT NULL DEFAULT
  (datetime('now','subsec'))` timestamps, `CREATE INDEX IF NOT EXISTS`, partial index `WHERE … IS NULL`.
- **Before:** (file does not exist)
- **After:** exact contents:
```sql
-- Node→hive ordered, acknowledged op-log outbox (SC2). One row per local write operation that must
-- propagate to the hive, drained in `seq` order and unacked-cleared only on a durable hive ack.
-- This is a SAFE TRACER BULLET: it runs ALONGSIDE the existing five legacy sync paths (additive); the
-- hive apply is idempotent so a double-apply across both channels is safe. Retiring the legacy paths
-- and adding non-task op types is a LATER increment.
CREATE TABLE IF NOT EXISTS node_outbox (
    id              BLOB PRIMARY KEY,
    -- Per-node monotonic sequence giving the node→hive total order. Assigned on INSERT via a SINGLE
    -- scalar-subquery statement `(SELECT COALESCE(MAX(seq),0)+1 FROM node_outbox)` (see 104), which is
    -- atomic under SQLite's single-writer lock. `UNIQUE` is the belt-and-suspenders guard: a duplicate
    -- seq from any concurrent two-step MAX(seq)+1 path fails loudly instead of corrupting the order
    -- (tournament R1/F4). Not a rowid alias (the PK is `id`).
    seq             INTEGER NOT NULL UNIQUE,
    op_type         TEXT NOT NULL,
    entity_type     TEXT NOT NULL,
    entity_id       BLOB NOT NULL,
    payload         TEXT NOT NULL,            -- JSON op payload
    idempotency_key TEXT NOT NULL UNIQUE,     -- hive dedup key, deterministic per write
    fencing_token   INTEGER,                  -- NULL in the tracer; populated by phase-2 fencing
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    acked_at        TEXT                      -- NULL until the hive durably acks this seq
);

-- Drain unacked ops in seq order: the streamer (107) selects WHERE acked_at IS NULL ORDER BY seq.
CREATE INDEX IF NOT EXISTS idx_node_outbox_unacked_seq
    ON node_outbox(seq)
    WHERE acked_at IS NULL;
```

## Allowed moves
ONLY create the one migration file with exactly the SQL above. Do NOT add Rust code, modify
`crates/db/src/models/mod.rs`, or touch any other migration. The `OutboxRepository` model + queries are
task 104; do not pre-create them.

## STOP triggers
- A migration with a timestamp ≥ `20260201000400` already exists → bump this file's timestamp to sort
  strictly last, and update the filename in `files:` to match (record the new name in the ledger).
- Any need to touch a file not in `files:`.

## Manual verification (record in decisions-ledger)
- `ls crates/db/migrations/20260201000400_add_node_outbox.sql` → exists and sorts last.
- The migration applies cleanly against a fresh DB: a `db::test_utils::create_test_pool()` build (run by
  `cargo test -p db --lib`) succeeds — the template DB applies ALL migrations, so a malformed migration
  fails the build. Record: `cargo test -p db --lib` exit status.
- `sqlite3 <test.db> ".schema node_outbox"` shows the table + `idx_node_outbox_unacked_seq` partial
  index. Record the schema dump.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db --lib" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 101` exits 0
