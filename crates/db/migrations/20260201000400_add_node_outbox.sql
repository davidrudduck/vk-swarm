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