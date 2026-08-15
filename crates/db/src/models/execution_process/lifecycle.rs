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

/// Attempt 2 (task 007), item 4: `task_attempts.executor` is nullable at the schema level
/// (legacy column, no `NOT NULL`) and sqlx's SQLite driver decodes a NULL into a plain `String`
/// target as `""` rather than erroring — confirmed empirically, not assumed. Decoding as
/// `Option<String>` and substituting THIS sentinel on `None` (rather than `unwrap_or_default()`)
/// keeps the emitted identity self-evidently a placeholder — lowercase and prose-shaped, unlike
/// every real value (`"CLAUDE_CODE"`, `"AMP"`, `"QA_MOCK"`, all `SCREAMING_SNAKE_CASE` per
/// migration `20250903091032`) — so a consumer or a human reading the journal cannot mistake it
/// for a real executor. Duplicated in `queries.rs` for the same reason `journal_err_to_sqlx` is.
const UNKNOWN_EXECUTOR: &str = "unknown (legacy NULL task_attempts.executor)";

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
    /// Task 007 attempt 2 (panels 17A/17B): gates emission on a real `running -> terminal`
    /// TRANSITION, in the UPDATE's own `WHERE ... AND status = 'running'` clause, with
    /// `RETURNING task_attempt_id` telling us in the SAME statement whether a transition actually
    /// happened. This single write-first statement satisfies BOTH constraints attempt 1 violated
    /// one of: (1) three identical `Completed` writes, or a `Completed -> Killed` write, now
    /// transition — and therefore emit — at most ONCE, because the second write's `WHERE` no
    /// longer matches (the row is no longer `'running'`); (2) the statement never opens the
    /// transaction as a READ that must later upgrade to a WRITE — 17B proved that shape earns a
    /// non-retryable `SQLITE_BUSY_SNAPSHOT` (517) under WAL (`no_read_then_upgrade` test below).
    /// 17A's own proposed fix (move a SELECT before the UPDATE) would have reintroduced exactly
    /// that hazard; this shape avoids the conflict entirely rather than picking a side.
    ///
    /// **Behavioural change from attempt 1 (and from pre-007):** an already-terminal row is no
    /// longer overwritten by a later call — 0 rows match, nothing is written, nothing is emitted.
    /// Checked all four production callers: `container.rs:562/:1572` and
    /// `local-deployment/container.rs:642` only ever reach this function while the row is still
    /// `running` (the last is additionally guarded by `was_stopped` for the `Killed`/`Completed`
    /// case, though not `Failed` — see the ledger residual on `was_stopped`'s TOCTOU window);
    /// `local-deployment/container.rs:2007` (`stop_execution`) only reaches this function while
    /// `get_child_from_store` still finds a tracked child, which the exit monitor removes only
    /// once it has itself written a terminal status. None appears to depend on re-overwriting an
    /// already-terminal row's `exit_code`/`completion_reason`/`completion_message`.
    ///
    /// The owning `TaskAttempt` is loaded via a SEPARATE read AFTER the write (not before it —
    /// see above), keyed off the `task_attempt_id` the UPDATE itself returned, to source
    /// `task_id` and executor identity — this row only carries `task_attempt_id`. Executor is
    /// decoded as `Option<String>` and substituted with [`UNKNOWN_EXECUTOR`] on NULL (item 4);
    /// see that constant's doc comment.
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
        // again below, by value, in the match that decides which event to emit — `.bind()` takes
        // its argument by value and `ExecutionProcessStatus` is not `Copy`.
        let status_for_write = status.clone();

        // Write-first: the FIRST statement this transaction issues is this UPDATE, not a SELECT
        // — SQLite's deferred-transaction machinery therefore acquires a RESERVED (write) lock
        // immediately rather than taking a read snapshot it would later need to upgrade. Runtime
        // API, not `query_as!`/`query_scalar!`: RETURNING a single column from an UPDATE isn't an
        // existing cached macro query, and this task's Change section forbids a new macro query.
        let transitioned_attempt_id: Option<Uuid> = sqlx::query_scalar(
            r#"UPDATE execution_processes
               SET status = ?, exit_code = ?, completed_at = ?, completion_reason = ?, completion_message = ?
               WHERE id = ? AND status = 'running'
               RETURNING task_attempt_id"#,
        )
        .bind(status_for_write)
        .bind(exit_code)
        .bind(completed_at)
        .bind(completion_reason)
        .bind(completion_message)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        // `transitioned_attempt_id` is `None` in exactly two cases, both meaning "no real
        // transition happened, emit nothing": `id` matched no row at all, OR `id` matched a row
        // that was already non-'running' (a repeat or contradictory terminal write). It is `Some`
        // iff this call just moved the row from `running` to `status`.
        if is_terminal && let Some(attempt_id) = transitioned_attempt_id {
            // A read AFTER the write (this transaction already holds the write lock from the
            // UPDATE above), so no upgrade hazard here either.
            let owner: Option<(Uuid, Option<String>)> =
                sqlx::query_as("SELECT task_id, executor FROM task_attempts WHERE id = ?")
                    .bind(attempt_id)
                    .fetch_optional(&mut *tx)
                    .await?;

            // `owner` being `None` here means the row transitioned (proven above) but its owning
            // `TaskAttempt` is gone — unreachable today (`task_attempts` FK is `ON DELETE
            // CASCADE`, so deleting the attempt would have deleted this execution_process row
            // too), but not something this function can rule out by construction, so it degrades
            // to "no event" rather than panicking or fabricating identity out of nothing.
            if let Some((task_id, executor)) = owner {
                let executor = executor.unwrap_or_else(|| {
                    tracing::warn!(
                        execution_process_id = %id,
                        attempt_id = %attempt_id,
                        "task_attempts.executor is NULL (legacy data) — emitting with a sentinel identity"
                    );
                    UNKNOWN_EXECUTOR.to_string()
                });

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
    use std::str::FromStr;

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

    // --- Attempt 2 (task 007, panels 17A/17B): transition-gating boundary tests ---
    //
    // 17A-2 proved the pre-attempt-2 guard was entirely untested: mutating `is_terminal` to
    // `true` (i.e. deleting the transition check) left the whole crate green. These three pin the
    // property no named test isolated.

    /// A `Running` write must never emit, regardless of the row's actual current status.
    #[tokio::test]
    async fn running_write_emits_nothing() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Running,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert!(
            rows.is_empty(),
            "a Running write must emit nothing: {rows:?}"
        );
    }

    /// Three identical `Completed` writes on the same process must emit exactly once — the
    /// second and third calls see a row that is no longer `'running'`, so their `WHERE` clause no
    /// longer matches and they become no-ops (bite-tested below: mutating the gate away turns
    /// this into 3 events, per 17A-1's own P1 finding).
    #[tokio::test]
    async fn repeated_identical_terminal_write_emits_once() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        for _ in 0..3 {
            ExecutionProcess::update_completion(
                &pool,
                process_id,
                ExecutionProcessStatus::Completed,
                Some(0),
                None,
                None,
            )
            .await
            .unwrap();
        }

        let rows = read_journal_rows(&pool).await;
        assert_eq!(
            rows.len(),
            1,
            "3 identical Completed writes on one process must emit exactly once: {rows:?}"
        );
        assert_eq!(rows[0].0, "attempt_finished");
    }

    /// `Completed -> Killed` must emit exactly once, not the two CONTRADICTORY events
    /// (`attempt_finished` then `attempt_failed` for the SAME process) 17A-1's P2 finding showed.
    /// The second call (`Killed`) sees a row that is no longer `'running'` (already `completed`)
    /// and becomes a no-op BOTH for the write and the event — the row's `status` column stays
    /// `completed`, it is not overwritten to `killed`. This is the declared behavioural change
    /// from pre-attempt-2 (ledger).
    #[tokio::test]
    async fn completed_then_killed_emits_once_not_two_contradictory_events() {
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
            None,
            None,
        )
        .await
        .unwrap();
        // Second call succeeds (Ok(())) — it just transitions nothing, per the WHERE clause.
        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Killed,
            None,
            Some("killed"),
            None,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(
            rows.len(),
            1,
            "Completed -> Killed must emit exactly once, not two contradictory events: {rows:?}"
        );
        assert_eq!(rows[0].0, "attempt_finished");

        let status: String =
            sqlx::query_scalar("SELECT status FROM execution_processes WHERE id = ?")
                .bind(process_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            status, "completed",
            "the second (no-op) call must not overwrite the row's status to 'killed'"
        );
    }

    /// Bite proof for the three tests above: without the transition gate, `update_completion`
    /// reproduces 17A-1's own findings exactly. Reconstructs the pre-attempt-2 (attempt 1) shape
    /// directly — SELECT owner (unconditionally), then UPDATE (unconditionally), emit whenever
    /// terminal — as a local closure, rather than editing production code via `.wai-scratch`
    /// swap-and-restore, since the property under test (gate presence) is naturally expressible
    /// as "does an unconditional-emit shape diverge from the real one," which is a cheaper and
    /// equally direct proof than a file-level revert.
    #[tokio::test]
    async fn bite_proof_ungated_shape_reproduces_17a1_p1_and_p2() {
        async fn update_completion_ungated(
            pool: &SqlitePool,
            id: Uuid,
            status: ExecutionProcessStatus,
            exit_code: Option<i64>,
        ) {
            let completed_at = Some(Utc::now());
            let mut tx = pool.begin().await.unwrap();
            sqlx::query(
                "UPDATE execution_processes SET status = ?, exit_code = ?, completed_at = ? WHERE id = ?",
            )
            .bind(status.clone())
            .bind(exit_code)
            .bind(completed_at)
            .bind(id)
            .execute(&mut *tx)
            .await
            .unwrap();
            let owner: (Uuid, Uuid, String) = sqlx::query_as(
                r#"SELECT ep.task_attempt_id, ta.task_id, ta.executor
                   FROM execution_processes ep JOIN task_attempts ta ON ta.id = ep.task_attempt_id
                   WHERE ep.id = ?"#,
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            let event = match (status, exit_code) {
                (ExecutionProcessStatus::Completed, Some(exit_code)) => {
                    NodeEvent::AttemptFinished {
                        task_id: owner.1,
                        attempt_id: owner.0,
                        execution_process_id: id,
                        executor: owner.2,
                        exit_code,
                    }
                }
                _ => NodeEvent::AttemptFailed {
                    task_id: owner.1,
                    attempt_id: owner.0,
                    execution_process_id: id,
                    executor: owner.2,
                    reason: "test".to_string(),
                },
            };
            event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;

        // P1: 3 identical Completed writes -> 3 events under the ungated shape.
        let attempt_p1 = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_p1 = seed_running_process(&pool, attempt_p1).await;
        for _ in 0..3 {
            update_completion_ungated(
                &pool,
                process_p1,
                ExecutionProcessStatus::Completed,
                Some(0),
            )
            .await;
        }
        let rows = read_journal_rows(&pool).await;
        assert_eq!(
            rows.iter().filter(|(t, _)| t == "attempt_finished").count(),
            3,
            "the ungated shape must reproduce 17A-1's P1 (3 events for 3 identical writes) — if \
             this fails, the bite proof itself is broken, not the real function"
        );

        // P2: Completed -> Killed -> BOTH attempt_finished and attempt_failed under the ungated shape.
        let attempt_p2 = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_p2 = seed_running_process(&pool, attempt_p2).await;
        update_completion_ungated(
            &pool,
            process_p2,
            ExecutionProcessStatus::Completed,
            Some(0),
        )
        .await;
        update_completion_ungated(&pool, process_p2, ExecutionProcessStatus::Killed, None).await;
        let rows = read_journal_rows(&pool).await;
        let p2_types: Vec<&str> = rows
            .iter()
            .filter(|(_, payload)| payload.contains(&process_p2.to_string()))
            .map(|(t, _)| t.as_str())
            .collect();
        assert_eq!(
            p2_types,
            vec!["attempt_finished", "attempt_failed"],
            "the ungated shape must reproduce 17A-1's P2 (one process emits BOTH events)"
        );
    }

    // --- Attempt 2 (task 007, item 5): rollback tests — 006 ships one per site, 007 shipped none. ---

    /// `update_completion`'s append happens inside the same transaction as the UPDATE; a failed
    /// append must roll back the status write too. Fault injection follows the pattern already
    /// established in `task/queries.rs`'s `lifecycle_event_tests`: rename `event_journal` out from
    /// under the append (DDL, done OUTSIDE any transaction so it survives/precedes cleanly).
    #[tokio::test]
    async fn update_completion_rolls_back_when_append_fails() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let result = ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Completed,
            Some(0),
            None,
            None,
        )
        .await;
        let err = result.expect_err("a failed journal append must surface as an error");
        assert!(
            format!("{err:?}").contains("event_journal"),
            "the failure must be the journal append, not an earlier statement: {err:?}"
        );

        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        let status: String =
            sqlx::query_scalar("SELECT status FROM execution_processes WHERE id = ?")
                .bind(process_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            status, "running",
            "a failed journal append must roll back the status write too"
        );
        let rows = read_journal_rows(&pool).await;
        assert!(rows.is_empty(), "no event may have landed: {rows:?}");
    }

    // --- Attempt 2 (task 007, item 4): NULL executor ---

    /// `task_attempts.executor` is nullable; a NULL must not silently emit `"executor": ""`
    /// (F17A-3/F17B-3). Proves the sentinel, not the decorative-but-unenforced assertion attempt
    /// 1 shipped (`!executor.is_empty()` against fixtures that always set it).
    #[tokio::test]
    async fn null_executor_emits_sentinel_not_empty_string() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;

        let attempt_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref) \
             VALUES (?, ?, NULL, 'b', 'main', '/tmp/wt')",
        )
        .bind(attempt_id)
        .bind(task_id)
        .execute(&pool)
        .await
        .unwrap();
        let process_id = seed_running_process(&pool, attempt_id).await;

        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Completed,
            Some(0),
            None,
            None,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(rows.len(), 1);
        let event: NodeEvent = serde_json::from_str(&rows[0].1).unwrap();
        match event {
            NodeEvent::AttemptFinished { executor, .. } => {
                assert_ne!(
                    executor, "",
                    "a NULL executor must not silently become an empty string"
                );
                assert_eq!(executor, UNKNOWN_EXECUTOR);
            }
            other => panic!("expected AttemptFinished, got {other:?}"),
        }
    }

    // --- Attempt 2 (task 007, panel 17B / THE CONFLICT): no read-then-upgrade ---

    fn is_busy_snapshot(err: &sqlx::Error) -> bool {
        err.as_database_error()
            .and_then(|e| e.code())
            .map(|c| c == "517")
            .unwrap_or(false)
    }

    /// Prod-like pool: WAL + `busy_timeout` + `max_connections(10)`, matching `crates/db/src/lib.rs`
    /// (not `test_utils::create_test_pool*`, which set neither `busy_timeout` nor >5 connections) —
    /// 17B's own harness pattern. A real file (not `:memory:`): WAL requires one.
    async fn build_contention_pool() -> (SqlitePool, tempfile::TempDir) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("contention.db");
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&format!(
            "sqlite://{}",
            db_path.display()
        ))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .min_connections(2)
            .max_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect_with(options)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, temp_dir)
    }

    /// REQUIRED by the attempt-2 amendment: proves the chosen shape does NOT read-then-upgrade.
    /// 200 independent processes, each transitioned by one real `update_completion` call, while a
    /// background writer commits to the SAME table every ~200µs (17B's own tight-loop
    /// methodology) for the whole run. The pre-007 (and pre-attempt-2) shape scored 0/200; this
    /// must too, because `update_completion`'s first and only write-adjacent statement is the
    /// UPDATE itself (see its doc comment) — no SELECT ever opens the transaction as a read.
    #[tokio::test]
    async fn update_completion_does_not_read_then_upgrade() {
        const ITERATIONS: usize = 200;

        let (pool, _tmp) = build_contention_pool().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;

        let mut process_ids = Vec::with_capacity(ITERATIONS);
        for _ in 0..ITERATIONS {
            let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
            process_ids.push(seed_running_process(&pool, attempt_id).await);
        }
        let decoy_attempt = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let decoy_process = seed_running_process(&pool, decoy_attempt).await;

        let writer_pool = pool.clone();
        let writer = tokio::spawn(async move {
            loop {
                let _ = sqlx::query(
                    "UPDATE execution_processes SET pid = COALESCE(pid, 0) + 1 WHERE id = ?",
                )
                .bind(decoy_process)
                .execute(&writer_pool)
                .await;
                tokio::time::sleep(std::time::Duration::from_micros(200)).await;
            }
        });

        let mut busy_snapshot_errors = 0usize;
        let mut other_errors = 0usize;
        for process_id in &process_ids {
            match ExecutionProcess::update_completion(
                &pool,
                *process_id,
                ExecutionProcessStatus::Completed,
                Some(0),
                None,
                None,
            )
            .await
            {
                Ok(()) => {}
                Err(e) if is_busy_snapshot(&e) => busy_snapshot_errors += 1,
                Err(_) => other_errors += 1,
            }
        }
        writer.abort();

        eprintln!(
            "no_read_then_upgrade(update_completion, real write-first shape): \
             {busy_snapshot_errors}/{ITERATIONS} SQLITE_BUSY_SNAPSHOT, {other_errors} other errors"
        );
        assert_eq!(
            busy_snapshot_errors, 0,
            "write-first update_completion must not read-then-upgrade under contention"
        );
        assert_eq!(
            other_errors, 0,
            "no other errors expected at this contention level"
        );
    }

    /// Calibration control for the test above: reconstructs attempt 1's REJECTED shape (SELECT
    /// before UPDATE, inside one deferred transaction — attempt 1's code, hand-rolled here since
    /// it is gone from the tree) against the IDENTICAL harness, to prove the harness is actually
    /// capable of reproducing panel 17B's finding rather than being silently toothless. If this
    /// control ever stopped producing failures, the test above would be proving nothing.
    #[tokio::test]
    async fn control_read_then_write_shape_reproduces_busy_snapshot() {
        const ITERATIONS: usize = 200;

        let (pool, _tmp) = build_contention_pool().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;

        let mut process_ids = Vec::with_capacity(ITERATIONS);
        for _ in 0..ITERATIONS {
            let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
            process_ids.push(seed_running_process(&pool, attempt_id).await);
        }
        let decoy_attempt = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let decoy_process = seed_running_process(&pool, decoy_attempt).await;

        let writer_pool = pool.clone();
        let writer = tokio::spawn(async move {
            loop {
                let _ = sqlx::query(
                    "UPDATE execution_processes SET pid = COALESCE(pid, 0) + 1 WHERE id = ?",
                )
                .bind(decoy_process)
                .execute(&writer_pool)
                .await;
                tokio::time::sleep(std::time::Duration::from_micros(200)).await;
            }
        });

        let mut busy_snapshot_errors = 0usize;
        for process_id in &process_ids {
            let mut tx = pool.begin().await.unwrap();
            // Attempt 1's shape: SELECT (read) first...
            let owner: Option<(Uuid,)> =
                sqlx::query_as("SELECT task_attempt_id FROM execution_processes WHERE id = ?")
                    .bind(process_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .unwrap();
            assert!(owner.is_some());

            // ...then UPDATE (write — the upgrade).
            let result =
                sqlx::query("UPDATE execution_processes SET status = 'completed' WHERE id = ?")
                    .bind(process_id)
                    .execute(&mut *tx)
                    .await;

            match result {
                Ok(_) => {
                    let _ = tx.commit().await;
                }
                Err(e) => {
                    if is_busy_snapshot(&e) {
                        busy_snapshot_errors += 1;
                    }
                    drop(tx);
                }
            }
        }
        writer.abort();

        eprintln!(
            "no_read_then_upgrade(control, attempt-1 read-then-write shape): \
             {busy_snapshot_errors}/{ITERATIONS} SQLITE_BUSY_SNAPSHOT"
        );
        assert!(
            busy_snapshot_errors > 0,
            "calibration control must reproduce at least one SQLITE_BUSY_SNAPSHOT — 0 here would \
             mean the harness cannot detect the hazard, and the real test above would be proving \
             nothing"
        );
    }
}
