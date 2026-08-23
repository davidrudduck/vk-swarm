//! Lifecycle and status update operations for execution processes.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::{ExecutionProcess, ExecutionProcessStatus};
use crate::models::event::NodeEvent;
use crate::models::event_journal;

use super::UNKNOWN_EXECUTOR;

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
    /// **Behavioural change from attempt 1 (and from pre-007), DELIBERATE and DECLARED (attempt
    /// 3, F18-1 — corrected from an inverted trace attempt 2 shipped here and in the ledger):** an
    /// already-terminal row is no longer overwritten by a later call — 0 rows match, nothing is
    /// written, nothing is emitted. `container.rs:562`/`:1572` only ever reach this function while
    /// the row is still `running`. `local-deployment/container.rs:642` (the exit monitor) is
    /// guarded by `!was_stopped(...)` for `Killed`/`Completed`, though not `Failed` — TOCTOU
    /// residual, ledger. **`local-deployment/container.rs:2007` (`stop_execution`) DOES reach this
    /// function on an already-terminal row, routinely, not only in a race:** the exit monitor
    /// writes the terminal status at `container.rs:642` but does not remove the child from
    /// `child_store` until `container.rs:918`, AFTER log normalization, a git commit
    /// (`try_commit_changes`), and MsgStore teardown — a wide window (the earlier "only reaches
    /// this function while `get_child_from_store` still finds a tracked child, which the exit
    /// monitor removes only once it has itself written a terminal status" claim had the
    /// implication backwards: the child STAYS findable through that whole window, it doesn't
    /// disappear). `stop_execution`'s own `get_child_from_store` call therefore SUCCEEDS in that
    /// window, and `routes/execution_processes.rs:192-201` imposes no status precondition before
    /// calling it — a user pressing Stop right after an agent finishes lands here.
    ///
    /// **The write and its event are silently discarded in that window, and this is accepted, not
    /// merely tolerated:** the alternative is dropping the transition gate and reintroducing
    /// F17A-1's duplicate/contradictory-event defect (`Completed` then `Killed` on one process).
    /// The pre-gate behaviour — overwriting a row that finished on its own to falsely claim it was
    /// user-`killed` — misreported what happened; this gate makes the row tell the truth about
    /// which outcome actually landed first, at the cost of losing the LATER call's own status.
    /// **User-visible consequence:** `ProcessesTab.tsx:286-296` renders `completion_reason` as a
    /// badge with `completion_message` as its tooltip; `ProjectTasks.tsx:142-166` shows an error
    /// banner when `status == 'failed'` (unconditionally) or `status == 'completed'` with
    /// `completion_reason` in `{eof, error, result_error}`. A Stop landing in the exit monitor's
    /// window used to leave `killed`/`'killed'` (banner suppressed for a `'completed'`-turned-
    /// `'killed'` row; for a `'failed'`-turned-`'killed'` row the banner was ALSO suppressed, since
    /// `'killed' != 'failed'`). It now leaves whatever the exit monitor already wrote (e.g.
    /// `failed`/`'eof'`) — banner shown (if the terminal status was `failed`), the Stop's own
    /// `completion_message` ("user pressed stop" or similar) never lands. Pinned by
    /// `stop_onto_already_terminal_row_discards_the_write_and_emits_nothing` below.
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
                    .map_err(sqlx::Error::from)?;
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

    /// Attempt 3, item 1 (F18-1): pins the DECLARED discard as a specified behaviour, not an
    /// emergent one. Models the actual production shape rather than a synthetic one — the exit
    /// monitor (`local-deployment/container.rs:642`) writes a terminal `Failed`/`"eof"` row, and
    /// `stop_execution` (`:2007`) can still reach `update_completion` up to 276 lines later
    /// (`child_store` removal happens only at `:918`) and calls it again with `Killed`. The second
    /// call's `status`, `completion_reason`, AND `completion_message` must all be discarded — not
    /// just `status` (the previous test only proved that much) — and it must emit no second event.
    #[tokio::test]
    async fn stop_onto_already_terminal_row_discards_the_write_and_emits_nothing() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;
        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;

        // The exit monitor's write: the agent disconnected without a Result message.
        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Failed,
            None,
            Some("eof"),
            None,
        )
        .await
        .unwrap();

        // stop_execution's write, landing in the window where child_store still finds the
        // (already-exited) child. Succeeds (Ok(())) but transitions nothing.
        ExecutionProcess::update_completion(
            &pool,
            process_id,
            ExecutionProcessStatus::Killed,
            None,
            Some("killed"),
            Some("user pressed stop"),
        )
        .await
        .unwrap();

        let row: (String, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT status, completion_reason, completion_message FROM execution_processes WHERE id = ?",
        )
        .bind(process_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            ("failed".to_string(), Some("eof".to_string()), None),
            "the Stop's status, completion_reason, AND completion_message must all be discarded \
             — the row must keep exactly what the exit monitor wrote"
        );

        let rows = read_journal_rows(&pool).await;
        assert_eq!(
            rows.len(),
            1,
            "the discarded Stop must emit no second event: {rows:?}"
        );
        assert_eq!(rows[0].0, "attempt_failed");
    }

    /// Bite proof for the three tests above: without the transition gate, `update_completion`
    /// reproduces 17A-1's own findings exactly. Reconstructs the pre-attempt-2 (attempt 1) shape
    /// directly — UPDATE (unconditionally), then SELECT owner (unconditionally), emit whenever
    /// terminal (corrected, attempt 4/F19-1: this previously said the SELECT came first, backwards
    /// both about attempt 1's actual write-first code AND about the closure directly below, which
    /// does UPDATE before SELECT) — as a local closure, rather than editing production code via
    /// `.wai-scratch` swap-and-restore, since the property under test (gate presence) is naturally
    /// expressible as "does an unconditional-emit shape diverge from the real one," which is a
    /// cheaper and equally direct proof than a file-level revert.
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

    // --- Attempt 2 (task 007, item 4)/attempt 3 (item 2): NULL executor ---

    /// De-tautologised sentinel assertion (attempt 3, F18-2): NOT `assert_eq!(executor,
    /// UNKNOWN_EXECUTOR)` — comparing the emitted value to the imported constant it was BUILT
    /// from is a tautology that passes even if the constant were changed to a real executor value
    /// (`"CLAUDE_CODE"`). Asserts the LITERAL instead, so drift between the two copies of the
    /// constant (`lifecycle.rs:32`/`queries.rs:31`) is caught by EITHER file's test — panel 19
    /// proved this discriminates by mutating each copy independently and observing disjoint
    /// failing test sets (ledger, attempt 4).
    ///
    /// Attempt 4/F19-2: previously ALSO asserted `executor.contains(' ')` as a second,
    /// independent "shape" discriminator (no real executor value has a space). Deleted: if the
    /// `assert_eq!` above passes, `executor` IS the literal, which already contains a space — so
    /// that assert could never fire, confirmed empirically (every panic under both sentinel
    /// mutations landed on the `assert_eq!` line, never this one). The underlying claim was true
    /// (panel 19 verified all ten `BaseCodingAgent` variants are space-free `SCREAMING_SNAKE_CASE`,
    /// every raw-string executor `INSERT` in the tree is `#[cfg(test)]`-only, and no migration can
    /// introduce a space) but dead-redundant as CODE once the literal is asserted first — not kept
    /// as inert documentation, to avoid presenting it as a second discriminator when it discriminates
    /// nothing.
    fn assert_is_unknown_executor_sentinel(executor: &str) {
        assert_eq!(
            executor, "unknown (legacy NULL task_attempts.executor)",
            "must match the sentinel LITERAL — comparing against the imported UNKNOWN_EXECUTOR \
             constant instead is what attempt 2 shipped, and it is a tautology: '{executor}'"
        );
    }

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
                assert_is_unknown_executor_sentinel(&executor);
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

    /// Scheduler-sensitive stress check for the real write-first `update_completion` shape.
    /// It transitions 200 independent processes while a background writer commits to the same
    /// table. A zero-error result is expected because the UPDATE is the transaction's first
    /// statement, but this timing-driven generator is supplemental evidence rather than a
    /// deterministic proof against every read-before-write mutation. The control below separately
    /// forces the hazardous SQLite schedule deterministically.
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

    /// Calibration control for the test above (attempt 3, F18-4: relabelled — the previous name
    /// and docstring claimed this reconstructs "attempt 1's REJECTED shape." **That was false.**
    /// `update_completion`'s attempt-1 code was ALREADY write-first (UPDATE first, owner SELECT
    /// after — THE CONFLICT section, task file), so it never had this hazard; the shape below is
    /// 17A's *proposed remediation* (read the prior status before the UPDATE to gate on a real
    /// transition), which is what panel 18 injected into the real function and scored 15/200 on
    /// (ledger). This control hand-reconstructs that SAME hypothetical shape, independently, to
    /// force SQLite's hazardous schedule directly. Panel 18's own injection-into-real-code result
    /// (C1) tested production code in a detached worktree but is not a shipped, repeatable test.
    /// This in-tree control proves the database failure mode independently; it does not calibrate
    /// the timing-driven production stress generator above or deterministically mutation-test the
    /// real function.
    #[tokio::test]
    async fn control_prior_status_read_reproduces_busy_snapshot() {
        let (pool, _tmp) = build_contention_pool().await;
        let project_id = seed_project(&pool).await;
        let task_id = seed_task(&pool, project_id).await;

        let attempt_id = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let process_id = seed_running_process(&pool, attempt_id).await;
        let decoy_attempt = seed_attempt(&pool, task_id, "CLAUDE_CODE").await;
        let decoy_process = seed_running_process(&pool, decoy_attempt).await;

        let mut tx = pool.begin().await.unwrap();
        // 17A's proposed remediation's shape (NOT attempt 1's — see this fn's docstring,
        // corrected attempt 4/F19-1): SELECT (read) first...
        let owner: Option<(Uuid,)> =
            sqlx::query_as("SELECT task_attempt_id FROM execution_processes WHERE id = ?")
                .bind(process_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap();
        assert!(owner.is_some());

        // Deterministically invalidate that read snapshot from another pooled connection.
        sqlx::query("UPDATE execution_processes SET pid = 1 WHERE id = ?")
            .bind(decoy_process)
            .execute(&pool)
            .await
            .unwrap();

        // ...then UPDATE (write — the upgrade).
        let error = sqlx::query("UPDATE execution_processes SET status = 'completed' WHERE id = ?")
            .bind(process_id)
            .execute(&mut *tx)
            .await
            .expect_err("the invalidated read snapshot must reject a write upgrade");

        assert!(
            is_busy_snapshot(&error),
            "calibration control must reproduce SQLITE_BUSY_SNAPSHOT, got {error}"
        );
    }
}
