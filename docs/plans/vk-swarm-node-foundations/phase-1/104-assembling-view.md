---
id: "104"
phase: 1
title: Add assembling view over attempts+processes+sessions + read accessor
status: ready
depends_on: ["103"]
parallel: false
conflicts_with: []
files:
  - crates/db/migrations/20260201000200_add_workstream_state_view.sql
  - crates/db/src/models/workstream_state.rs
  - crates/db/src/models/mod.rs
irreversible: false
scope_test: "crates/db/src/models/workstream_state.rs"
allowed_change: mixed
covers_criteria: [SC3]
---
## Failing test (write first)
In `crates/db/src/models/workstream_state.rs` add `#[cfg(test)] mod tests` with:
```rust
#[tokio::test]
async fn test_workstream_state_assembles_the_triple() {
    let (pool, _tmp) = crate::test_utils::create_test_pool().await;
    // Seed project → task → task_attempt (container_ref set) → execution_process (running, codingagent)
    // → executor_session (session_id = "sess-123"). Use existing db test seed helpers where available.
    let attempt_id = seed_running_attempt_with_session(&pool, "sess-123").await; // see STOP triggers
    let rows = WorkstreamState::find_by_task_attempt(&pool, attempt_id).await.unwrap();
    let row = rows.first().expect("one assembled row");
    assert_eq!(row.session_id.as_deref(), Some("sess-123"));
    assert!(row.container_ref.is_some());          // worktree pointer present
    assert_eq!(row.status, "running");
    // resume_state defaults to NULL until recovery classifies it
    assert!(row.resume_state.is_none());
}
```

## Change
- **File:** `crates/db/migrations/20260201000200_add_workstream_state_view.sql` (NEW). Timestamp sorts
  after 103. Read-only assembling VIEW (SC3b) joining the run-state triple:
```sql
-- Read-only assembling view over the run-state triple (task_attempts + execution_processes +
-- executor_sessions). The durable, queryable "workstream-state surface" recovery resumes from and
-- downstream phases (P3/P6) query (SC3). No new run entity; this is a projection of existing tables.
CREATE VIEW IF NOT EXISTS v_workstream_state AS
SELECT
    ep.id                AS execution_process_id,
    ep.task_attempt_id   AS task_attempt_id,
    ta.container_ref     AS container_ref,
    ta.branch            AS branch,
    ta.target_branch     AS target_branch,
    ep.run_reason        AS run_reason,
    ep.status            AS status,
    ep.resume_state      AS resume_state,
    ep.pid               AS pid,
    ep.before_head_commit AS before_head_commit,
    ep.after_head_commit  AS after_head_commit,
    es.session_id        AS session_id,
    ep.created_at        AS created_at
FROM execution_processes ep
JOIN task_attempts ta ON ep.task_attempt_id = ta.id
LEFT JOIN executor_sessions es ON es.execution_process_id = ep.id;
```
- **File:** `crates/db/src/models/workstream_state.rs` (NEW). A `WorkstreamState` struct (plain fields,
  String/Option<String>/Option<i64> — do NOT reuse the typed enums to keep the view query simple) +
  `find_by_task_attempt(pool, attempt_id) -> Result<Vec<Self>, sqlx::Error>` selecting from
  `v_workstream_state ORDER BY created_at DESC`, and `find_resumable_running(pool) -> Result<Vec<Self>,
  sqlx::Error>` (`WHERE status = 'running'`). Include the test above.
- **File:** `crates/db/src/models/mod.rs`
- **Anchor:** the `pub mod …;` block (L17-32), alphabetical neighbourhood near `pub mod webhook;`.
- **Before:** `pub mod webhook;`
- **After:** `pub mod webhook;\npub mod workstream_state;`

## Allowed moves
Create the view migration, the new `workstream_state` model (read-only accessors + test), and register
the module in `models/mod.rs`. Do NOT modify `ExecutionProcess`, `TaskAttempt`, `ExecutorSession`, or
their queries. The view is READ-ONLY (no INSERT/UPDATE through it).

## Sibling alignment
`crates/db/src/models/workstream_state.rs` is a new model module. Read an existing simple sibling model
(e.g. `crates/db/src/models/webhook.rs` or `executor_session.rs`): match its `query_as!` column-alias
style (`col as "name!: Type"`), its error type (`sqlx::Error` vs a module error enum), module-doc
header, and test conventions (`create_test_pool`). Justify any divergence in the ledger.

## STOP triggers
- No reusable seed helper exists for a running attempt + session → write a local `seed_running_attempt
  _with_session` in the test module inserting the full chain (project→task→task_attempt→
  execution_process→executor_session). Record it.
- `query_as!` against `v_workstream_state` fails to compile (schema not materialized) → export
  `DATABASE_URL` to a dev DB with 101/103/104 migrations applied so `query_as!` checks the live schema
  (Trap 2). Do NOT `cargo sqlx prepare` in this task (it churns the tracked `.sqlx` cache the gate
  rejects; regen is a `/wai:close` step).
- A column referenced in the view (`ta.branch`, `es.session_id`, `ep.resume_state`) does not exist with
  that exact name → STOP and reconcile against the real schema before finalizing.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db workstream_state" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 104` exits 0
