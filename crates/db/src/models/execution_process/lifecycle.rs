//! Lifecycle and status update operations for execution processes.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::{ExecutionProcess, ExecutionProcessStatus};
use crate::models::event::NodeEvent;
use crate::models::event_journal::{self, EventJournalError};

/// Map a journal-append failure onto `sqlx::Error` — duplicated from
/// `execution_process::queries::journal_err_to_sqlx` (that copy is private to its module, and
/// this task's file set does not include `mod.rs`, so there is nowhere shared to put one copy).
/// See `task::queries::journal_err_to_sqlx`'s doc comment for the Database/Serde split rationale.
fn journal_err_to_sqlx(e: EventJournalError) -> sqlx::Error {
    match e {
        EventJournalError::Database(err) => err,
        EventJournalError::Serde(err) => {
            sqlx::Error::Protocol(format!("event journal payload serialization failed: {err}"))
        }
    }
}

impl ExecutionProcess {
    pub async fn was_stopped(pool: &SqlitePool, id: Uuid) -> bool {
        if let Ok(exp_process) = Self::find_by_id(pool, id).await
            && exp_process.is_some_and(|ep| {
                ep.status == ExecutionProcessStatus::Killed
                    || ep.status == ExecutionProcessStatus::Completed
            })
        {
            return true;
        }
        false
    }

    /// Update execution process status and completion info.
    ///
    /// Task 007: wrapped in a transaction that appends `attempt_finished`/`attempt_failed` to the
    /// event journal on a terminal transition (`status` != `Running`; only that branch computes a
    /// `completed_at`, mirroring the terminality check this function already made). The owning
    /// `TaskAttempt` is loaded INSIDE the transaction (same reasoning as `ExecutionProcess::create`)
    /// to source `task_id`, `attempt_id`, and executor identity — this row only carries
    /// `task_attempt_id`.
    pub async fn update_completion(
        pool: &SqlitePool,
        id: Uuid,
        status: ExecutionProcessStatus,
        exit_code: Option<i64>,
        completion_reason: Option<&str>,
        completion_message: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let is_terminal = !matches!(status, ExecutionProcessStatus::Running);
        let completed_at = if is_terminal { Some(Utc::now()) } else { None };

        let mut tx = pool.begin().await?;

        // Cloned into a binding (not passed as an inline `.clone()`, which the macro's generated
        // code borrows as a temporary that would then be dropped too early): `status` is used
        // again below, by value, in the match that decides which event to emit — `query!`'s
        // `.bind()` takes its argument by value and `ExecutionProcessStatus` is not `Copy`.
        let status_for_write = status.clone();
        sqlx::query!(
            r#"UPDATE execution_processes
               SET status = $1, exit_code = $2, completed_at = $3, completion_reason = $4, completion_message = $5
               WHERE id = $6"#,
            status_for_write,
            exit_code,
            completed_at,
            completion_reason,
            completion_message,
            id
        )
        .execute(&mut *tx)
        .await?;

        if is_terminal {
            // Runtime API: new SQL text, not a re-use of an existing macro query.
            let owner: Option<(Uuid, Uuid, String)> = sqlx::query_as(
                r#"SELECT ep.task_attempt_id, ta.task_id, ta.executor
                   FROM execution_processes ep
                   JOIN task_attempts ta ON ta.id = ep.task_attempt_id
                   WHERE ep.id = ?"#,
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

            // `owner` is None only when `id` did not match any row (the UPDATE above then also
            // affected nothing) — no event for an update on a nonexistent execution process.
            if let Some((attempt_id, task_id, executor)) = owner {
                // Matches on `&status`, not `status` by value: the fallback arm below still
                // needs `status` (for its Debug-formatted reason string) after the match, and
                // matching a tuple literal `(status, exit_code)` would move `status` into that
                // temporary before any arm runs.
                let event = match (&status, exit_code) {
                    (ExecutionProcessStatus::Completed, Some(exit_code)) => {
                        NodeEvent::AttemptFinished {
                            task_id,
                            attempt_id,
                            execution_process_id: id,
                            executor,
                            exit_code,
                        }
                    }
                    // `exit_code` is `Option<i64>` at the source but `i64` on `AttemptFinished` —
                    // a `None` here on the success transition must NOT be papered over with
                    // `unwrap_or(0)` (that would report a clean exit that never happened). Emit
                    // `attempt_failed` naming the missing exit code instead.
                    (ExecutionProcessStatus::Completed, None) => NodeEvent::AttemptFailed {
                        task_id,
                        attempt_id,
                        execution_process_id: id,
                        executor,
                        reason: "execution process completed with no exit code recorded"
                            .to_string(),
                    },
                    (_, _) => {
                        let reason = completion_reason
                            .or(completion_message)
                            .map(str::to_string)
                            .unwrap_or_else(|| {
                                format!(
                                    "execution process ended with status {status:?} and no \
                                     completion reason recorded"
                                )
                            });
                        NodeEvent::AttemptFailed {
                            task_id,
                            attempt_id,
                            execution_process_id: id,
                            executor,
                            reason,
                        }
                    }
                };
                event_journal::append(&mut *tx, &event)
                    .await
                    .map_err(journal_err_to_sqlx)?;
            }
        }

        tx.commit().await?;

        Ok(())
    }

    /// Update the "after" commit oid for the process
    pub async fn update_after_head_commit(
        pool: &SqlitePool,
        id: Uuid,
        after_head_commit: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE execution_processes
               SET after_head_commit = $1
               WHERE id = $2"#,
            after_head_commit,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update the "before" commit oid for the process
    pub async fn update_before_head_commit(
        pool: &SqlitePool,
        id: Uuid,
        before_head_commit: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE execution_processes
               SET before_head_commit = $1
               WHERE id = $2"#,
            before_head_commit,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update the system process ID (PID) for process tree discovery
    pub async fn update_pid(pool: &SqlitePool, id: Uuid, pid: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE execution_processes
               SET pid = $1
               WHERE id = $2"#,
            pid,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Set restore boundary: drop processes newer than the specified process, undrop older/equal
    pub async fn set_restore_boundary(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
        boundary_process_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        // Monotonic drop: only mark newer records as dropped; never undrop.
        sqlx::query!(
            r#"UPDATE execution_processes
               SET dropped = TRUE
             WHERE task_attempt_id = $1
               AND created_at > (SELECT created_at FROM execution_processes WHERE id = $2)
               AND dropped = FALSE
            "#,
            task_attempt_id,
            boundary_process_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Soft-drop processes at and after the specified boundary (inclusive)
    pub async fn drop_at_and_after(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
        boundary_process_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"UPDATE execution_processes
               SET dropped = TRUE
             WHERE task_attempt_id = $1
               AND created_at >= (SELECT created_at FROM execution_processes WHERE id = $2)
               AND dropped = FALSE"#,
            task_attempt_id,
            boundary_process_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }
}

/// Task 007: attempt lifecycle events emitted from `ExecutionProcess::update_completion` — the
/// terminal completion write. Uses `create_test_pool_with_migrations` per the task file's
/// dictate, mirroring task 006's `lifecycle_event_tests` module in `task/queries.rs`.
#[cfg(test)]
mod lifecycle_event_tests {
    use super::*;
    use crate::models::event::NodeEvent;
    use crate::test_utils::create_test_pool_with_migrations;

    async fn seed_project(pool: &SqlitePool) -> Uuid {
        let pid = Uuid::new_v4();
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (?, 'p', '/tmp/p')")
            .bind(pid)
            .execute(pool)
            .await
            .unwrap();
        pid
    }

    async fn seed_task(pool: &SqlitePool, project_id: Uuid) -> Uuid {
        let tid = Uuid::new_v4();
        sqlx::query("INSERT INTO tasks (id, project_id, title, status) VALUES (?, ?, 't', 'todo')")
            .bind(tid)
            .bind(project_id)
            .execute(pool)
            .await
            .unwrap();
        tid
    }

    async fn seed_attempt(pool: &SqlitePool, task_id: Uuid, executor: &str) -> Uuid {
        let aid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref) \
             VALUES (?, ?, ?, 'b', 'main', '/tmp/wt')",
        )
        .bind(aid)
        .bind(task_id)
        .bind(executor)
        .execute(pool)
        .await
        .unwrap();
        aid
    }

    /// Inserted directly (not via `ExecutionProcess::create`) so the journal contains ONLY the
    /// `update_completion` events under test, with no `attempt_started` noise.
    async fn seed_running_process(pool: &SqlitePool, attempt_id: Uuid) -> Uuid {
        let pid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO execution_processes (id, task_attempt_id, status, run_reason, executor_action) \
             VALUES (?, ?, 'running', 'codingagent', '{}')",
        )
        .bind(pid)
        .bind(attempt_id)
        .execute(pool)
        .await
        .unwrap();
        pid
    }

    async fn read_journal_rows(pool: &SqlitePool) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT event_type, payload FROM event_journal ORDER BY seq",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    /// Test 2 (task file).
    #[tokio::test]
    async fn completion_success_emits_attempt_finished() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Completed,
            Some(0),
            Some("result_success"),
            None,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(rows.len(), 1, "exactly one event_journal row");
        assert_eq!(rows[0].0, "attempt_finished");
        let event: NodeEvent = serde_json::from_str(&rows[0].1).unwrap();
        match event {
            NodeEvent::AttemptFinished {
                task_id: tid,
                attempt_id: aid,
                execution_process_id: epid,
                executor,
                exit_code,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(aid, attempt_id);
                assert_eq!(epid, process_id);
                assert_eq!(executor, "CLAUDE_CODE");
                assert_eq!(exit_code, 0);
            }
            other => panic!("expected AttemptFinished, got {other:?}"),
        }
    }

    /// Test 3 (task file).
    #[tokio::test]
    async fn completion_failure_emits_attempt_failed() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Failed,
            Some(1),
            Some("result_error"),
            Some("agent errored"),
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(rows.len(), 1, "exactly one event_journal row");
        assert_eq!(rows[0].0, "attempt_failed");
        let event: NodeEvent = serde_json::from_str(&rows[0].1).unwrap();
        match event {
            NodeEvent::AttemptFailed {
                task_id: tid,
                attempt_id: aid,
                execution_process_id: epid,
                executor,
                reason,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(aid, attempt_id);
                assert_eq!(epid, process_id);
                assert_eq!(executor, "CLAUDE_CODE");
                assert_eq!(
                    reason, "result_error",
                    "completion_reason is the preferred reason source"
                );
            }
            other => panic!("expected AttemptFailed, got {other:?}"),
        }
    }

    /// Test 4 (task file). Drives an intermediate update (`update_pid`, one of the five
    /// non-instrumented UPDATE statements in this file) and asserts it emits no `attempt_%`
    /// event — guards against emitting on every UPDATE, not only the terminal completion write.
    #[tokio::test]
    async fn non_terminal_update_emits_nothing() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        ExecutionProcess::update_pid(&pool, process_id, 12345)
            .await
            .unwrap();

        let rows = read_journal_rows(&pool).await;
        let attempt_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t.starts_with("attempt_"))
            .collect();
        assert!(
            attempt_rows.is_empty(),
            "a non-terminal UPDATE must emit no attempt_% event: {attempt_rows:?}"
        );
    }

    /// Test 7 (task file). SC2 names executor identity on all three attempt events; this proves
    /// it is populated on BOTH terminal variants, not only on `attempt_started`.
    #[tokio::test]
    async fn terminal_events_carry_executor_identity() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;

        let finished_attempt = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let finished_process = seed_running_process(&pool, finished_attempt).await;
        ExecutionProcess::update_completion(
            &pool,
            finished_process,
            ExecutionProcessStatus::Completed,
            Some(0),
            None,
            None,
        )
        .await
        .unwrap();

        let failed_attempt = seed_attempt(&pool, task_id, "AMP").await;
        let failed_process = seed_running_process(&pool, failed_attempt).await;
        ExecutionProcess::update_completion(
            &pool,
            failed_process,
            ExecutionProcessStatus::Failed,
            None,
            Some("error"),
            None,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(rows.len(), 2);
        for (event_type, payload) in &rows {
            let event: NodeEvent = serde_json::from_str(payload).unwrap();
            let executor = match &event {
                NodeEvent::AttemptFinished { executor, .. } => executor,
                NodeEvent::AttemptFailed { executor, .. } => executor,
                other => panic!("expected a terminal attempt event, got {other:?}"),
            };
            assert!(
                !executor.is_empty(),
                "{event_type} must carry non-empty executor identity"
            );
        }
    }

    /// Supplemental — not one of the task file's seven named tests. Proves the exit-code-width
    /// guard from the Change section: a `Completed` transition with `exit_code = None` must NOT
    /// paper over the gap with `unwrap_or(0)` — it must emit `attempt_failed`, not a fabricated
    /// `attempt_finished { exit_code: 0 }`.
    #[tokio::test]
    async fn completed_with_missing_exit_code_emits_attempt_failed_not_a_fabricated_zero() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Completed,
            None, // exit_code missing despite a success status
            None,
            None,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].0, "attempt_failed",
            "a Completed transition with no exit_code must not fabricate an attempt_finished"
        );
        let event: NodeEvent = serde_json::from_str(&rows[0].1).unwrap();
        match event {
            NodeEvent::AttemptFailed { reason, .. } => {
                assert!(
                    reason.to_lowercase().contains("exit code"),
                    "reason must name the missing exit code: {reason}"
                );
            }
            other => panic!("expected AttemptFailed, got {other:?}"),
        }
    }
}
