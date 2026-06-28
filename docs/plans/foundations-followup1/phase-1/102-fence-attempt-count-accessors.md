---
id: "102"
phase: 1
title: Add increment/get fence_attempt_count scalar accessors + test
status: passed
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - crates/db/src/models/execution_process/queries.rs
irreversible: false
scope_test: "crates/db/src/models/execution_process/queries.rs"
allowed_change: edit
covers_criteria: [SC2b]
---
## Failing test (write first)
Add to the existing `#[cfg(test)] mod tests` block at the bottom of
`crates/db/src/models/execution_process/queries.rs` (the module already exists — see the
`test_find_session_id_before_process_*` tests). Mirror the seeding style of
`cleanup_orphan_executions_accessor_set_and_get_resume_state` in
`crates/services/src/services/container.rs` (raw `sqlx::query()` inserts).

```rust
#[tokio::test]
async fn fence_attempt_count_increments_and_reads_back() {
    use db::test_utils::create_test_pool;
    let (pool, _tmp) = create_test_pool().await;

    // Minimal FK chain: project -> task -> task_attempt -> execution_process
    let project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES ($1, 'p', '/tmp/p')")
        .bind(project_id).execute(&pool).await.unwrap();
    let task_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO tasks (id, project_id, title, status) VALUES ($1, $2, 't', 'todo')")
        .bind(task_id).bind(project_id).execute(&pool).await.unwrap();
    let attempt_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref) VALUES ($1, $2, 'QA_MOCK', 'b', 'main', '/tmp/wt')")
        .bind(attempt_id).bind(task_id).execute(&pool).await.unwrap();
    let process_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO execution_processes (id, task_attempt_id, run_reason, executor_action, status, started_at) VALUES ($1, $2, 'codingagent', '{}', 'running', datetime('now'))")
        .bind(process_id).bind(attempt_id).execute(&pool).await.unwrap();

    // Default is 0
    assert_eq!(ExecutionProcess::get_fence_attempt_count(&pool, process_id).await.unwrap(), 0);

    // Increment three times
    for expected in 1..=3 {
        ExecutionProcess::increment_fence_attempt_count(&pool, process_id).await.unwrap();
        assert_eq!(ExecutionProcess::get_fence_attempt_count(&pool, process_id).await.unwrap(), expected);
    }
}
```

NOTE: `cargo test -p db` compiles `query!` macros against the live schema. Export
`DATABASE_URL=sqlite://$(pwd)/dev_assets/db.sqlite` (with task 101's migration applied) before the
gate runs, OR rely on the dev server having auto-migrated. Do NOT run `cargo sqlx prepare`
(decisions-ledger Trap 2).

## Change
For each file in files::
- **File:** `crates/db/src/models/execution_process/queries.rs`
- **Anchor:** immediately AFTER the existing `get_resume_state` function (it ends ~line 161; it is a
  `pub async fn get_resume_state(pool: &SqlitePool, id: Uuid) -> Result<Option<String>, sqlx::Error>`
  doing a single `SELECT resume_state … WHERE id = ?`). Add the two functions inside the same
  `impl ExecutionProcess { … }` block.
- **Before:** (end of `get_resume_state`, no accessor for fence_attempt_count exists)
- **After:** insert these two functions:
  ```rust
  /// Increment the fence-attempt counter for a process stuck in D-state (CouldNotKill path
  /// of crash recovery). Persisted so the count survives the server restarts that D-state
  /// forces. See ADR-0005.
  pub async fn increment_fence_attempt_count(
      pool: &SqlitePool,
      id: Uuid,
  ) -> Result<(), sqlx::Error> {
      sqlx::query!(
          "UPDATE execution_processes SET fence_attempt_count = fence_attempt_count + 1 WHERE id = ?",
          id
      )
      .execute(pool)
      .await
      .map(|_| ())
  }

  /// Read the current fence-attempt counter for a process. Returns 0 for rows that have
  /// never hit the CouldNotKill path (column default).
  pub async fn get_fence_attempt_count(pool: &SqlitePool, id: Uuid) -> Result<i64, sqlx::Error> {
      sqlx::query_scalar!(
          r#"SELECT fence_attempt_count FROM execution_processes WHERE id = ?"#,
          id
      )
      .fetch_one(pool)
      .await
  }
  ```

## Allowed moves
- Add ONLY the two functions above + the one test. Do NOT add `fence_attempt_count` to the
  `ExecutionProcess` struct or to any existing `query_as!(ExecutionProcess, …)` SELECT (the column
  is accessed via these dedicated scalars only — same pattern as `resume_state`, per Phase 2a
  decisions-ledger "resume-intent column accessed via dedicated scalar queries").
- Do NOT run `cargo sqlx prepare`.

## STOP triggers
- `get_resume_state` is not where stated, or the `impl ExecutionProcess` block cannot be found.
- `cargo check -p db` fails with "no such column: fence_attempt_count" → task 101's migration is
  not applied to the DB `DATABASE_URL` points at. Apply it (do NOT regenerate `.sqlx`).
- Functions named `increment_fence_attempt_count`/`get_fence_attempt_count` already exist.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db fence_attempt_count_increments_and_reads_back" bash ~/.claude/wai/scripts/task-gate.sh foundations-followup1 102` exits 0
