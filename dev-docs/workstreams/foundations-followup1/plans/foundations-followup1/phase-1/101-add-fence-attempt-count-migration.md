---
id: "101"
phase: 1
title: Add fence_attempt_count column migration on execution_processes
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/db/migrations/20260201000300_add_fence_attempt_count.sql
irreversible: true
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC2b]
---
## Failing test (write first)
N/A — migration-only task; verified by `## Manual verification` below. (Column behaviour is exercised
by task 102's accessor test once the column exists.)

## Change
For each file in files::
- **File:** `crates/db/migrations/20260201000300_add_fence_attempt_count.sql` (NEW)
- **Anchor:** new migration file. Timestamp `20260201000300` sorts immediately AFTER the current
  latest migration `20260201000200_add_workstream_state_view.sql` (verified: it is the highest
  timestamp under `crates/db/migrations/`). No collision.
- **Before:** (file does not exist)
- **After:** exact file contents:
  ```sql
  -- Add a counter tracking how many times crash-recovery has tried and failed to fence
  -- a process stuck in D-state (uninterruptible sleep). Read/incremented only on the
  -- CouldNotKill path of cleanup_orphan_executions; surfaced for operator escalation.
  -- See ADR-0005.
  ALTER TABLE execution_processes ADD COLUMN fence_attempt_count INTEGER NOT NULL DEFAULT 0;
  ```

## Allowed moves
- Create ONLY the migration file above. Do NOT modify the `ExecutionProcess` struct, any
  `query_as!(ExecutionProcess, …)` SELECT, or any other migration.
- Do NOT add an index (the column is read/written by `id` only, on the cold crash-recovery path).
- Do NOT run `cargo sqlx prepare` (decisions-ledger Trap 2 — `.sqlx` regeneration is a single
  `/wai:close` step, NOT part of this task).

## STOP triggers
- A migration with timestamp ≥ `20260201000300` already exists.
- `execution_processes` already has a `fence_attempt_count` column (grep
  `crates/db/migrations/` first — must be zero hits).
- The migration fails to apply cleanly to a fresh dev DB.

## Manual verification (record in decisions-ledger)
Run against a live dev DB and record output:
1. `DATABASE_URL=sqlite://$(pwd)/dev_assets/db.sqlite sqlx migrate run --source crates/db/migrations`
   → expect `Applied 20260201000300/migrate add fence attempt count`.
2. `sqlite3 dev_assets/db.sqlite ".schema execution_processes" | grep fence_attempt_count`
   → expect `fence_attempt_count INTEGER NOT NULL DEFAULT 0`.
3. `cargo test -p db --lib` → exits 0 (template-DB migration tests still pass).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db --lib" bash ~/.claude/wai/scripts/task-gate.sh foundations-followup1 101` exits 0
