---
id: "103"
phase: 1
title: Add resume-intent column migration on execution_processes
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/db/migrations/20260201000100_add_resume_state_to_execution_processes.sql
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC3]
---
## Failing test (write first)
N/A — covered by: task 104's view test and task 304's recovery-classification tests, both of which
read/write this column via the scalar accessors added in 304. This task is a pure additive migration.

## Change
- **File:** `crates/db/migrations/20260201000100_add_resume_state_to_execution_processes.sql` (NEW)
- **Anchor:** new migration; timestamp must sort AFTER 101's `20260201000000_add_queued_messages.sql`.
- **Before:** (file does not exist)
- **After:** exact contents:
```sql
-- Resume-intent marker for crash recovery (ADR-0001 fence-then-resume / ADR-0003).
-- NULL  = not yet classified by recovery (the default for all existing + new rows).
-- 'pending'  = recovery has decided this running row should be resumed (--resume).
-- 'resumed'  = recovery has re-spawned it.
-- 'abandoned'= recovery classified it unrecoverable (no session / non-resumable executor).
-- Accessed via dedicated scalar queries (NOT the ExecutionProcess FromRow struct) to avoid
-- editing every existing query_as! SELECT (see decisions-ledger: SQLx column-access decision).
ALTER TABLE execution_processes ADD COLUMN resume_state TEXT;

-- Recovery scans running rows needing classification; partial index keeps it cheap.
CREATE INDEX IF NOT EXISTS idx_execution_processes_resume_state
    ON execution_processes(resume_state)
    WHERE status = 'running';
```

## Allowed moves
ONLY create this one migration file with exactly the SQL above. Do NOT add `resume_state` to the
`ExecutionProcess` struct or any `query_as!` SELECT (that ripple is deliberately avoided — see ledger).
Do NOT modify any model or other migration.

## STOP triggers
- A migration with timestamp ≥ this one already exists → bump the timestamp to sort strictly after 101
  and update `files:` + the ledger.
- A `resume_state` (or similarly-named) column already exists on `execution_processes` → STOP; the
  column may have been added by another change (`grep -rn "resume_state" crates/db/migrations/`).
- Any need to touch a Rust file.

## Manual verification (record in decisions-ledger)
- `ls crates/db/migrations/20260201000100_add_resume_state_to_execution_processes.sql` → exists, sorts
  after 101.
- The migration applies cleanly: `cargo test -p db --lib` (builds a test pool that applies all
  migrations) succeeds. Record the exit status.
- `sqlite3 <test.db> ".schema execution_processes"` shows the new `resume_state TEXT` column and the
  `idx_execution_processes_resume_state` partial index. Record the relevant lines.
- Confirm NO `query_as!(ExecutionProcess, …)` SELECT was modified (`git diff --stat` touches only the
  migration file) — the column is intentionally NOT added to the FromRow struct.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db --lib" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 103` exits 0
