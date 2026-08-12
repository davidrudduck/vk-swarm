-- Node-local durable event journal (ADR-0017). One row per covered lifecycle change, written in the
-- same transaction as the discrete state-write statement it describes. `seq` is the cursor every
-- consumer resumes from.
--
-- Divergence from the node_outbox sibling (20260201000400): that table assigns `seq` by scalar
-- subquery with a UNIQUE guard because its PK is `id BLOB` and `seq` cannot be a rowid alias. Here
-- `seq` IS the primary key, so AUTOINCREMENT is both correct and cheaper. AUTOINCREMENT also
-- guarantees no reuse after deletion, which compaction requires.
CREATE TABLE IF NOT EXISTS event_journal (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    payload    TEXT NOT NULL,             -- JSON of the typed NodeEvent enum
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Consumers filter by type and always read in seq order from a cursor.
CREATE INDEX IF NOT EXISTS idx_event_journal_type_seq ON event_journal(event_type, seq);

-- Per-hook cursor so trigger-hook processing survives restarts (at-least-once, ADR-0017 D3).
-- Compaction normally never deletes rows at or above MIN(last_processed_seq) across this table.
-- `needs_rebootstrap` is how a hook learns it lost that protection: when the journal exceeds
-- VK_EVENT_MAX_ROWS the hard cap overrides the cursor floor (D6, revised), and every cursor the
-- deletion passed is flagged here so the hook sees explicit loss instead of silently resuming
-- mid-gap.
CREATE TABLE IF NOT EXISTS trigger_cursors (
    hook_name          TEXT PRIMARY KEY,
    last_processed_seq INTEGER NOT NULL DEFAULT 0,
    needs_rebootstrap  INTEGER NOT NULL DEFAULT 0,
    updated_at         TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);
