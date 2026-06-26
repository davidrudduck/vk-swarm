---
id: "101"
phase: 1
title: Add queued_messages table migration
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/db/migrations/20260201000000_add_queued_messages.sql
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC2]
---
## Failing test (write first)
N/A — covered by: task 102's `queued_messages` round-trip test (which runs against this migration via
`db::test_utils::create_test_pool()`, applying all migrations). This task is a pure additive schema
migration with no Rust code; its correctness is proven when 102's persistence test goes green. The
migration itself is exercised by every test pool build.

## Change
- **File:** `crates/db/migrations/20260201000000_add_queued_messages.sql` (NEW)
- **Anchor:** new migration file; timestamp prefix must sort AFTER the latest existing migration
  (`20260131000000_add_webhooks.sql`). Use `20260201000000` (or a later UTC stamp if a newer migration
  has landed — STOP-check below).
- **Before:** (file does not exist)
- **After:** exact contents:
```sql
-- Persist the follow-up message queue to SQLite so queued prompts survive a crash/restart.
-- Backs MessageQueueStore (was an in-memory HashMap<Uuid, Vec<QueuedMessage>>). One row per
-- queued message; ordering within a task attempt is by `position` (0-based, contiguous).
CREATE TABLE IF NOT EXISTS queued_messages (
    id              BLOB PRIMARY KEY,
    task_attempt_id BLOB NOT NULL,
    content         TEXT NOT NULL,
    variant         TEXT,
    position        INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (task_attempt_id) REFERENCES task_attempts(id) ON DELETE CASCADE
);

-- Drain/peek/list all key on task_attempt_id and order by position.
CREATE INDEX IF NOT EXISTS idx_queued_messages_attempt_position
    ON queued_messages(task_attempt_id, position);
```

## Allowed moves
ONLY create the one migration file with exactly the SQL above. Do NOT modify `message_queue.rs` (that
is task 102), any other migration, or any model. Do NOT add Rust code.

## STOP triggers
- A migration with a timestamp ≥ `20260201000000` already exists → bump this file's timestamp to sort
  strictly last, and update the filename in `files:` to match (record the new name in the ledger).
- `task_attempts` does not have an `id BLOB PRIMARY KEY` (verify: `sqlx`/schema uses BLOB for UUIDs in
  this repo — confirm against an existing migration before finalizing the FK column type). If UUIDs are
  stored as TEXT here, switch `id`/`task_attempt_id` to `TEXT` to match. Verify with:
  `grep -rn "REFERENCES task_attempts" crates/db/migrations/`.
- Any need to touch a file not in `files:`.

## Manual verification (record in decisions-ledger)
- `ls crates/db/migrations/20260201000000_add_queued_messages.sql` → exists and sorts last.
- The migration applies cleanly against a fresh DB: a `db::test_utils::create_test_pool()` build (run by
  `cargo test -p db --lib`) succeeds — the template DB applies ALL migrations, so a malformed migration
  fails the build. Record: `cargo test -p db --lib` exit status.
- `sqlite3 <test.db> ".schema queued_messages"` (or equivalent) shows the table + the
  `idx_queued_messages_attempt_position` index. Record the schema dump.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db --lib" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 101` exits 0
