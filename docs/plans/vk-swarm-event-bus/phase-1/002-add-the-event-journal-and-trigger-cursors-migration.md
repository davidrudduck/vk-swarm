---
id: "002"
phase: 1
title: "Add the event_journal and trigger_cursors migration"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - "crates/db/migrations/20260812000000_add_event_journal.sql"
siblings: ["crates/db/migrations/20260201000400_add_node_outbox.sql"]
irreversible: false
scope_test: "crates/db"
allowed_change: create
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — a migration has no unit test of its own; it is proved by task 004's model tests,
which run against `db::test_utils::create_test_pool_with_migrations()` and would fail to compile/query
without these tables. Verified here by the manual schema check below.


## Change
**File:** `crates/db/migrations/20260812000000_add_event_journal.sql`
**Anchor:** new file
**Sibling to read FIRST:** `crates/db/migrations/20260201000400_add_node_outbox.sql`. Note its
seq-assignment reasoning: it uses a scalar subquery `(SELECT COALESCE(MAX(seq),0)+1 FROM node_outbox)`
with a `UNIQUE` guard *because its primary key is `id BLOB`, so `seq` cannot be a rowid alias*. Here
`seq` IS the primary key, so `INTEGER PRIMARY KEY AUTOINCREMENT` is the correct divergence — record
that justification in the decisions-ledger per the sibling-alignment rule.

**After:**
```sql
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
```


## Allowed moves
ONLY create this one migration file. Do NOT edit any existing migration (they are
applied and immutable). Do NOT add Rust code — that is tasks 003/004.


## STOP triggers
- A migration with a timestamp >= 20260812000000 already exists — pick the next free timestamp and
  say so in the ledger.
- `sqlx migrate run` reports a checksum mismatch on any EXISTING migration — STOP, that means an
  applied migration was edited.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db"

1. `cargo sqlx migrate run` (or start the dev server) against a scratch DB.
2. `sqlite3 <db> ".schema event_journal"` shows `seq INTEGER PRIMARY KEY AUTOINCREMENT`.
3. `sqlite3 <db> ".schema trigger_cursors"` shows `hook_name TEXT PRIMARY KEY` and
   `needs_rebootstrap INTEGER NOT NULL DEFAULT 0`.
4. `sqlite3 <db> "select name from sqlite_master where type='index' and name='idx_event_journal_type_seq'"`
   returns one row.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 002` exits 0
