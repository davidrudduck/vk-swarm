//! CRUD query operations for tasks.

use chrono::{DateTime, Utc};
use sqlx::{Acquire, Executor, Sqlite, SqlitePool};
use uuid::Uuid;

use super::{CreateTask, Task, TaskStatus, TaskWithAttemptStatus};
use crate::models::event::NodeEvent;
use crate::models::event_journal::{self, EventJournalError};
use crate::models::project::Project;

/// Map a journal-append failure onto `sqlx::Error` so the four task lifecycle functions can stay
/// on their pre-existing `Result<_, sqlx::Error>` signatures (task 006 forbids changing return
/// types). `Database` unwraps directly; `Serde` (payload serialization) has no sqlx::Error
/// analogue, so it is reported via `Protocol` — the same pattern this crate already uses at
/// node_outbox.rs:79 and task_breakdown/queries.rs:235 for folding a non-sqlx failure into a
/// sqlx::Error-only signature.
fn journal_err_to_sqlx(e: EventJournalError) -> sqlx::Error {
    match e {
        EventJournalError::Database(err) => err,
        EventJournalError::Serde(err) => {
            sqlx::Error::Protocol(format!("event journal payload serialization failed: {err}"))
        }
    }
}

impl Task {
    pub async fn parent_project(&self, pool: &SqlitePool) -> Result<Option<Project>, sqlx::Error> {
        Project::find_by_id(pool, self.project_id).await
    }

    pub async fn find_by_project_id_with_attempt_status(
        pool: &SqlitePool,
        project_id: Uuid,
        include_archived: bool,
    ) -> Result<Vec<TaskWithAttemptStatus>, sqlx::Error> {
        let records = sqlx::query!(
            r#"SELECT
  t.id                            AS "id!: Uuid",
  t.project_id                    AS "project_id!: Uuid",
  t.title,
  t.description,
  t.status                        AS "status!: TaskStatus",
  t.parent_task_id                AS "parent_task_id: Uuid",
  t.shared_task_id                AS "shared_task_id: Uuid",
  t.created_at                    AS "created_at!: DateTime<Utc>",
  t.updated_at                    AS "updated_at!: DateTime<Utc>",
  t.remote_assignee_user_id       AS "remote_assignee_user_id: Uuid",
  t.remote_assignee_name,
  t.remote_assignee_username,
  t.remote_version                AS "remote_version!: i64",
  t.remote_last_synced_at         AS "remote_last_synced_at: DateTime<Utc>",
  t.remote_stream_node_id         AS "remote_stream_node_id: Uuid",
  t.remote_stream_url,
  t.archived_at                   AS "archived_at: DateTime<Utc>",
  t.activity_at                   AS "activity_at: DateTime<Utc>",

  CASE WHEN EXISTS (
    SELECT 1
      FROM task_attempts ta
      JOIN execution_processes ep
        ON ep.task_attempt_id = ta.id
     WHERE ta.task_id       = t.id
       AND ep.status        = 'running'
       AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     LIMIT 1
  ) THEN 1 ELSE 0 END            AS "has_in_progress_attempt!: i64",

  CASE WHEN (
    SELECT ep.status
      FROM task_attempts ta
      JOIN execution_processes ep
        ON ep.task_attempt_id = ta.id
     WHERE ta.task_id       = t.id
     AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     ORDER BY ep.created_at DESC
     LIMIT 1
  ) IN ('failed','killed') THEN 1 ELSE 0 END
                                 AS "last_attempt_failed!: i64",

  ( SELECT ta.executor
      FROM task_attempts ta
      WHERE ta.task_id = t.id
     ORDER BY ta.created_at DESC
      LIMIT 1
    )                               AS "executor!: String",

  (SELECT MAX(ep.started_at)
     FROM task_attempts ta
     JOIN execution_processes ep ON ep.task_attempt_id = ta.id
    WHERE ta.task_id = t.id
      AND ep.run_reason = 'codingagent'
      AND ep.dropped = FALSE
  )                                 AS "latest_execution_started_at: DateTime<Utc>",

  (SELECT MAX(ep.completed_at)
     FROM task_attempts ta
     JOIN execution_processes ep ON ep.task_attempt_id = ta.id
    WHERE ta.task_id = t.id
      AND ep.run_reason = 'codingagent'
      AND ep.dropped = FALSE
      AND ep.completed_at IS NOT NULL
  )                                 AS "latest_execution_completed_at: DateTime<Utc>",

  p.source_node_name

FROM tasks t
LEFT JOIN projects p ON p.id = t.project_id
WHERE t.project_id = $1
  AND (t.archived_at IS NULL OR $2)
  AND (
    t.remote_last_synced_at IS NULL
    OR EXISTS (SELECT 1 FROM task_attempts ta WHERE ta.task_id = t.id)
  )
ORDER BY COALESCE(t.activity_at, t.created_at) DESC"#,
            project_id,
            include_archived
        )
        .fetch_all(pool)
        .await?;

        let tasks = records
            .into_iter()
            .map(|rec| TaskWithAttemptStatus {
                task: Task {
                    id: rec.id,
                    project_id: rec.project_id,
                    title: rec.title,
                    description: rec.description,
                    status: rec.status,
                    parent_task_id: rec.parent_task_id,
                    shared_task_id: rec.shared_task_id,
                    created_at: rec.created_at,
                    updated_at: rec.updated_at,
                    remote_assignee_user_id: rec.remote_assignee_user_id,
                    remote_assignee_name: rec.remote_assignee_name,
                    remote_assignee_username: rec.remote_assignee_username,
                    remote_version: rec.remote_version,
                    remote_last_synced_at: rec.remote_last_synced_at,
                    remote_stream_node_id: rec.remote_stream_node_id,
                    remote_stream_url: rec.remote_stream_url,
                    archived_at: rec.archived_at,
                    activity_at: rec.activity_at,
                },
                has_in_progress_attempt: rec.has_in_progress_attempt != 0,
                has_merged_attempt: false, // TODO use merges table
                last_attempt_failed: rec.last_attempt_failed != 0,
                executor: rec.executor,
                latest_execution_started_at: rec.latest_execution_started_at,
                latest_execution_completed_at: rec.latest_execution_completed_at,
                source_node_name: rec.source_node_name,
            })
            .collect();

        Ok(tasks)
    }

    /// Fetch a single task with its attempt status.
    ///
    /// Equivalent to `find_by_project_id_with_attempt_status` filtered to one task.
    /// Uses two cached queries rather than one new macro to preserve the .sqlx cache.
    pub async fn find_by_id_with_attempt_status(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<TaskWithAttemptStatus>, sqlx::Error> {
        // Step 1: resolve project_id (needed for the attempt-status query)
        let task = Self::find_by_id(pool, id).await?;
        let Some(task) = task else {
            return Ok(None);
        };
        // Step 2: fetch all tasks for the project with status, then pick ours
        let tasks =
            Self::find_by_project_id_with_attempt_status(pool, task.project_id, true).await?;
        Ok(tasks.into_iter().find(|t| t.task.id == id))
    }

    // NOTE: A single-query version of find_by_id_with_attempt_status exists as a draft
    // but requires running `cargo sqlx prepare` to cache it. Add it here once the
    // .sqlx cache is regenerated (run: DATABASE_URL=sqlite:dev_assets/db.sqlite cargo sqlx prepare)
    #[allow(dead_code)]
    fn _single_query_reminder() {}

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                      remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                      remote_assignee_name,
                      remote_assignee_username,
                      remote_version as "remote_version!: i64",
                      remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                      remote_stream_node_id as "remote_stream_node_id: Uuid",
                      remote_stream_url,
                      archived_at as "archived_at: DateTime<Utc>",
                      activity_at as "activity_at: DateTime<Utc>"
               FROM tasks
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_rowid(pool: &SqlitePool, rowid: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                      remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                      remote_assignee_name,
                      remote_assignee_username,
                      remote_version as "remote_version!: i64",
                      remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                      remote_stream_node_id as "remote_stream_node_id: Uuid",
                      remote_stream_url,
                      archived_at as "archived_at: DateTime<Utc>",
                      activity_at as "activity_at: DateTime<Utc>"
               FROM tasks
               WHERE rowid = $1"#,
            rowid
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id_and_project_id(
        pool: &SqlitePool,
        id: Uuid,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                      remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                      remote_assignee_name,
                      remote_assignee_username,
                      remote_version as "remote_version!: i64",
                      remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                      remote_stream_node_id as "remote_stream_node_id: Uuid",
                      remote_stream_url,
                      archived_at as "archived_at: DateTime<Utc>",
                      activity_at as "activity_at: DateTime<Utc>"
               FROM tasks
               WHERE id = $1 AND project_id = $2"#,
            id,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_shared_task_id<'e, E>(
        executor: E,
        shared_task_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                      remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                      remote_assignee_name,
                      remote_assignee_username,
                      remote_version as "remote_version!: i64",
                      remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                      remote_stream_node_id as "remote_stream_node_id: Uuid",
                      remote_stream_url,
                      archived_at as "archived_at: DateTime<Utc>",
                      activity_at as "activity_at: DateTime<Utc>"
               FROM tasks
               WHERE shared_task_id = $1
               LIMIT 1"#,
            shared_task_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTask,
        task_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let status = data.status.clone().unwrap_or_default();
        let mut tx = pool.begin().await?;
        let task = sqlx::query_as!(
            Task,
            r#"INSERT INTO tasks (id, project_id, title, description, status, parent_task_id, shared_task_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                         remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                         remote_assignee_name,
                         remote_assignee_username,
                         remote_version as "remote_version!: i64",
                         remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                         remote_stream_node_id as "remote_stream_node_id: Uuid",
                         remote_stream_url,
                         archived_at as "archived_at: DateTime<Utc>",
                         activity_at as "activity_at: DateTime<Utc>""#,
            task_id,
            data.project_id,
            data.title,
            data.description,
            status,
            data.parent_task_id,
            data.shared_task_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let event = NodeEvent::TaskCreated {
            task_id: task.id,
            project_id: task.project_id,
        };
        event_journal::append(&mut *tx, &event)
            .await
            .map_err(journal_err_to_sqlx)?;

        tx.commit().await?;

        // Best-effort SC2 tracer op — deliberately OUTSIDE the transaction, see
        // enqueue_task_upsert_op's doc comment: a failed enqueue must not roll back the task write.
        Self::enqueue_task_upsert_op(pool, &task).await;
        Ok(task)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        project_id: Uuid,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        parent_task_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        let mut tx = pool.begin().await?;

        // Read the prior status INSIDE the transaction — reading it before begin() (or via a
        // separate pool connection) would race a concurrent writer and could report a stale
        // old_status once this transaction's UPDATE actually lands.
        let old_status: Option<TaskStatus> = sqlx::query_scalar::<_, TaskStatus>(
            "SELECT status FROM tasks WHERE id = ? AND project_id = ?",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(&mut *tx)
        .await?;

        let task = sqlx::query_as!(
            Task,
            r#"UPDATE tasks
               SET title = $3, description = $4, status = $5, parent_task_id = $6
               WHERE id = $1 AND project_id = $2
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                         remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                         remote_assignee_name,
                         remote_assignee_username,
                         remote_version as "remote_version!: i64",
                         remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                         remote_stream_node_id as "remote_stream_node_id: Uuid",
                         remote_stream_url,
                         archived_at as "archived_at: DateTime<Utc>",
                         activity_at as "activity_at: DateTime<Utc>""#,
            id,
            project_id,
            title,
            description,
            status,
            parent_task_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Emit task_status_changed ONLY when the status actually differs — a title/description-only
        // update must not produce a status event.
        if let Some(old_status) = old_status
            && old_status != task.status
        {
            let event = NodeEvent::TaskStatusChanged {
                task_id: task.id,
                old_status,
                new_status: task.status.clone(),
            };
            event_journal::append(&mut *tx, &event)
                .await
                .map_err(journal_err_to_sqlx)?;
        }

        tx.commit().await?;

        Self::enqueue_task_upsert_op(pool, &task).await;
        Ok(task)
    }

    /// Enqueue a `task.upsert` op into node_outbox alongside the local write (SC2 tracer).
    /// Runs ALONGSIDE the legacy hive_sync path (additive; hive apply is idempotent). Best-effort:
    /// a failed enqueue is logged, NOT propagated — the legacy path remains the backstop, and the
    /// enqueue is a separate statement from the task write (not one txn), so a crash between them is
    /// covered by the legacy sync. (Threading a shared txn through all Task::create callers is OUT of
    /// scope for the tracer — see decisions-ledger.)
    async fn enqueue_task_upsert_op(pool: &SqlitePool, task: &Task) {
        use crate::models::node_outbox::{NewOutboxOp, OutboxRepository};
        let payload = match serde_json::to_value(task) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, task_id = %task.id, "skip outbox enqueue: serialize failed");
                return;
            }
        };
        // Per-write-unique idempotency key. DELIBERATELY NOT `task:{id}:{version}`: Task::update does
        // NOT bump any version column (queries.rs UPDATE sets only title/description/status/parent_task_id),
        // so a version-only key collides on every update and the UNIQUE(idempotency_key) constraint
        // would silently drop the update op. A fresh Uuid suffix is assigned ONCE here and persisted
        // with the row, so a re-transmit of the SAME outbox row reuses the SAME key and the hive dedups
        // (node_op_log PK). The hive also applies idempotently on (source_node_id, source_task_id), so
        // distinct keys across writes of the same task are safe. "Deterministic" is not an SC
        // requirement — only per-write uniqueness + stable-per-row.
        let op = NewOutboxOp {
            op_type: "task.upsert".to_string(),
            entity_type: "task".to_string(),
            entity_id: task.id,
            payload,
            idempotency_key: format!("task:{}:{}", task.id, Uuid::new_v4()),
            fencing_token: None,
        };
        if let Err(e) = OutboxRepository::enqueue_op(pool, op).await {
            tracing::warn!(error = %e, task_id = %task.id, "failed to enqueue task.upsert op (legacy sync is the backstop)");
        }
    }

    /// Delete a task and journal `TaskDeleted` atomically, on whatever executor it is given.
    ///
    /// THREE production call sites, not one, and not all inside a caller-owned transaction:
    /// `core.rs:663` passes `&mut *tx` (an outer transaction spanning
    /// `nullify_children_by_parent_id` + this delete); `remote.rs:254` and `:266` both pass the
    /// bare pool. Bound is `Acquire`, not bare `Executor`: delete needs THREE sequential
    /// statements (DELETE...RETURNING, conditionally append, commit) against the one executor it
    /// was given, and a bare `E: Executor` value is consumed by each call with no way to reborrow
    /// an opaque generic — `Acquire::begin()` returns an owned `Transaction` that CAN be
    /// reborrowed via `&mut *tx` for each statement.
    ///
    /// `begin`, not `acquire`: on `&mut Transaction` (already inside a transaction) `begin()`
    /// opens a `SAVEPOINT` nested in the caller's transaction — its own `commit()` below is a
    /// `RELEASE SAVEPOINT`, not a real commit, so the caller's outer `tx.commit()` is still what
    /// makes it durable, and a savepoint on the SAME already-open connection acquires no new lock
    /// (the hazard an earlier STOP trigger, written for a single-caller model, was actually
    /// guarding against). On a bare pool `begin()` opens a REAL transaction, closing the gap
    /// where three separate autocommit statements could leave a deleted task with no journal row
    /// (`sqlx-core-0.8.6/transaction.rs:277-291`, depth-keyed per-connection in
    /// `sqlx-sqlite-0.8.6/connection/worker.rs`).
    ///
    /// Plain `async fn`, not the `impl Future` + split-lifetime shape an earlier revision of this
    /// function needed: THAT shape was required by `.acquire()`, not `.begin()`. `.acquire()`'s
    /// `Acquire::Connection = &'c mut SqliteConnection` is a BORROW that carries the bound's `'c`
    /// lifetime through the reborrow into the returned future, which is what forced an `async
    /// fn`'s single elided lifetime to serve two masters and tripped "implementation of
    /// `sqlx::Acquire` is not general enough" at the axum caller. `.begin()` returns an OWNED
    /// `Transaction<'c, _>` value instead — nothing borrows `executor` past that point, so there
    /// is no reborrow-through-a-lifetime for the HRTB solver to fail to prove. Re-verified after
    /// the switch (this collapse), not assumed from the switch alone: `cargo check --workspace
    /// --all-targets` is clean, see `_assert_delete_future_is_send` below for the compile-time
    /// net.
    pub async fn delete<'c, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error>
    where
        E: Acquire<'c, Database = Sqlite> + Send,
    {
        let mut tx = executor.begin().await?;

        // Write-first, one round trip: `DELETE ... RETURNING` gives both "was a row actually
        // deleted" AND the project_id the event payload needs, in the SAME statement. Replaces an
        // earlier SELECT-then-DELETE shape that cost a real error rate on the POOL path: `.begin()`
        // there opens SQLite's deferred transaction mode, which starts as a read and must upgrade
        // to a write on the first write statement — and SQLite's busy handler does NOT retry that
        // upgrade (only a fresh write-lock acquisition), so a `SELECT` immediately followed by a
        // `DELETE` measured 6 failures in 40 under contention where writing first measured 0.
        // Atomicity held in every run either way (this was an error-rate cost, not a torn write),
        // but write-first removes the read-then-upgrade shape entirely rather than merely
        // tolerating it. Runtime API, not a macro: new SQL text needs `cargo sqlx prepare`, which
        // this crate's `.sqlx` offline cache setup cannot support (see this task's own Change
        // section).
        let project_id: Option<Uuid> =
            sqlx::query_scalar::<_, Uuid>("DELETE FROM tasks WHERE id = ? RETURNING project_id")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;

        // A row was deleted iff RETURNING produced one. No separate rows_affected() check needed
        // — a delete of a nonexistent id must not fabricate an event, and project_id being None
        // already means nothing was deleted.
        let rows_affected = if let Some(project_id) = project_id {
            let event = NodeEvent::TaskDeleted {
                task_id: id,
                project_id,
            };
            event_journal::append(&mut *tx, &event)
                .await
                .map_err(journal_err_to_sqlx)?;
            1
        } else {
            0
        };

        tx.commit().await?;

        Ok(rows_affected)
    }

    /// Compile-time Send net for `delete`'s plain `async fn` shape (attempt-2 remediation, item
    /// 6). `async fn` INFERS `Send` for its returned future from what the body captures; the
    /// earlier `impl Future + Send + 'a` shape ASSERTED it explicitly. Without this check, a
    /// future change that makes `delete`'s captured state non-Send would fail to compile not
    /// here, at the source, but at whichever caller's own Send-bounded context (e.g. axum's
    /// `Handler` trait) happens to need it — the exact opaque, hard-to-diagnose failure this
    /// function turns into a clear one, located at the right place.
    #[allow(dead_code)]
    fn _assert_delete_future_is_send(conn: &mut sqlx::SqliteConnection, id: Uuid) {
        fn assert_send<F: Send>(_: &F) {}
        assert_send(&Task::delete(conn, id));
    }

    pub async fn exists(
        pool: &SqlitePool,
        id: Uuid,
        project_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT id as \"id!: Uuid\" FROM tasks WHERE id = $1 AND project_id = $2",
            id,
            project_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(result.is_some())
    }

    /// All swarm-linked tasks (`shared_task_id IS NOT NULL`) with their `remote_version`, for the SC5
    /// anti-entropy digest. Read-only; ordered by `id` for a stable digest. NO `limit` cap — the digest
    /// MUST cover EVERY swarm-linked task in one shot so the hive can detect divergence on any task
    /// (a `limit` would silently truncate the digest and leave divergences undetected past the batch
    /// boundary with no cursor/pagination to advance). The node's swarm-linked task count is bounded by
    /// its local `tasks` table, so the unbounded read is acceptable. The `archived_at IS NULL` filter
    /// was REMOVED to align with the "all swarm-linked tasks" requirement: an archived task that still
    /// carries a `shared_task_id` is still part of the swarm link, and the hive must see it in the
    /// digest to detect if the hive lost it (hive-has/node-lacks divergence includes archived tasks).
    pub async fn find_digest_entries(pool: &SqlitePool) -> Result<Vec<TaskDigestRow>, sqlx::Error> {
        sqlx::query!(
            r#"SELECT id as "id!: Uuid", remote_version as "remote_version!: i64"
               FROM tasks
               WHERE shared_task_id IS NOT NULL
               ORDER BY id ASC"#,
        )
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| TaskDigestRow {
                    id: r.id,
                    remote_version: r.remote_version,
                })
                .collect()
        })
    }
}

/// A node-side anti-entropy digest row: the id-bridge key + the version the node believes the hive holds.
#[derive(Debug, Clone)]
pub struct TaskDigestRow {
    pub id: Uuid,
    pub remote_version: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        project::{CreateProject, Project},
        task::tests::setup_test_pool,
    };

    #[tokio::test]
    async fn test_task_crud() {
        let (pool, _temp_dir) = setup_test_pool().await;

        // Create project
        let project_id = Uuid::new_v4();
        let project_data = CreateProject {
            name: "Test Project".to_string(),
            git_repo_path: format!("/tmp/test-repo-{}", project_id),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        let _project = Project::create(&pool, &project_data, project_id)
            .await
            .expect("Failed to create project");

        // Create task
        let task_id = Uuid::new_v4();
        let task_data = CreateTask::from_title_description(
            project_id,
            "Test Task".to_string(),
            Some("Description".to_string()),
        );
        let task = Task::create(&pool, &task_data, task_id)
            .await
            .expect("Failed to create task");

        assert_eq!(task.id, task_id);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Description".to_string()));
        assert_eq!(task.status, TaskStatus::Todo);

        // Find by ID
        let found = Task::find_by_id(&pool, task_id)
            .await
            .expect("Query failed")
            .expect("Task not found");
        assert_eq!(found.id, task_id);

        // Find by ID and project_id
        let found = Task::find_by_id_and_project_id(&pool, task_id, project_id)
            .await
            .expect("Query failed")
            .expect("Task not found");
        assert_eq!(found.id, task_id);

        // Exists check
        let exists = Task::exists(&pool, task_id, project_id)
            .await
            .expect("Query failed");
        assert!(exists);

        // Update
        let updated = Task::update(
            &pool,
            task_id,
            project_id,
            "Updated Title".to_string(),
            Some("Updated Desc".to_string()),
            TaskStatus::InProgress,
            None,
        )
        .await
        .expect("Update failed");
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.status, TaskStatus::InProgress);

        // Delete
        let deleted = Task::delete(&pool, task_id).await.expect("Delete failed");
        assert_eq!(deleted, 1);

        // Verify deleted
        let found = Task::find_by_id(&pool, task_id)
            .await
            .expect("Query failed");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_project_id_with_attempt_status() {
        let (pool, _temp_dir) = setup_test_pool().await;

        // Create project
        let project_id = Uuid::new_v4();
        let project_data = CreateProject {
            name: "Test Project".to_string(),
            git_repo_path: format!("/tmp/test-repo-{}", project_id),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        let _project = Project::create(&pool, &project_data, project_id)
            .await
            .expect("Failed to create project");

        // Create tasks
        for i in 0..3 {
            let task_id = Uuid::new_v4();
            let task_data =
                CreateTask::from_title_description(project_id, format!("Task {}", i), None);
            Task::create(&pool, &task_data, task_id)
                .await
                .expect("Failed to create task");
        }

        // Query with attempt status
        let tasks = Task::find_by_project_id_with_attempt_status(&pool, project_id, false)
            .await
            .expect("Query failed");

        assert_eq!(tasks.len(), 3);
        for task in &tasks {
            assert_eq!(task.project_id, project_id);
            assert!(!task.has_in_progress_attempt);
            assert!(!task.has_merged_attempt);
            assert!(!task.last_attempt_failed);
        }
    }
}

#[cfg(test)]
mod outbox_enqueue_tests {
    use super::*;
    use crate::models::node_outbox::OutboxRepository;
    use crate::test_utils::create_test_pool;

    async fn seed_project(pool: &SqlitePool) -> Uuid {
        let pid = Uuid::new_v4();
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (?, 'p', '/tmp/p')")
            .bind(pid)
            .execute(pool)
            .await
            .unwrap();
        pid
    }

    #[tokio::test]
    async fn create_then_update_enqueues_two_ordered_task_upsert_ops() {
        let (pool, _tmp) = create_test_pool().await;
        let project_id = seed_project(&pool).await;

        let task_id = Uuid::new_v4();
        let created = Task::create(
            &pool,
            &CreateTask {
                project_id,
                title: "t1".into(),
                description: None,
                status: None,
                parent_task_id: None,
                image_ids: None,
                shared_task_id: None,
            },
            task_id,
        )
        .await
        .unwrap();

        Task::update(
            &pool,
            created.id,
            project_id,
            "t2".into(),
            None,
            TaskStatus::InProgress,
            None,
        )
        .await
        .unwrap();

        let ops = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
        assert_eq!(ops.len(), 2, "create + update each enqueue one op");
        assert!(ops.iter().all(|o| o.op_type == "task.upsert"));
        assert!(ops.iter().all(|o| o.entity_type == "task"));
        assert!(ops.iter().all(|o| o.entity_id == task_id));
        assert!(ops[1].seq > ops[0].seq, "causal order preserved");
        assert_ne!(ops[0].idempotency_key, ops[1].idempotency_key);
    }

    #[tokio::test]
    async fn find_digest_entries_returns_only_swarm_linked_tasks_with_version() {
        let (pool, _tmp) = create_test_pool().await;
        let project_id = seed_project(&pool).await;

        let linked_id = Uuid::new_v4();
        let linked = Task::create(
            &pool,
            &CreateTask {
                project_id,
                title: "linked".into(),
                description: None,
                status: None,
                parent_task_id: None,
                image_ids: None,
                shared_task_id: Some(Uuid::new_v4()),
            },
            linked_id,
        )
        .await
        .unwrap();

        sqlx::query("UPDATE tasks SET remote_version = 3 WHERE id = ?")
            .bind(linked.id)
            .execute(&pool)
            .await
            .unwrap();

        let _unlinked = Task::create(
            &pool,
            &CreateTask {
                project_id,
                title: "unlinked".into(),
                description: None,
                status: None,
                parent_task_id: None,
                image_ids: None,
                shared_task_id: None,
            },
            Uuid::new_v4(),
        )
        .await
        .unwrap();

        let entries = Task::find_digest_entries(&pool).await.unwrap();
        assert_eq!(
            entries.len(),
            1,
            "only the swarm-linked (shared_task_id IS NOT NULL) task is in the digest"
        );
        assert_eq!(
            entries[0].remote_version, 3,
            "version is the task's remote_version"
        );
        assert_eq!(
            entries[0].id, linked_id,
            "entity_id == the linked task's LOCAL id"
        );
    }
}

/// Task 006: task lifecycle events (create/update/update_status/delete) landing in the event
/// journal, inside the same transaction as the state write. Uses
/// `create_test_pool_with_migrations` per the task file's dictate (not the template-copy
/// `create_test_pool`, so each test starts from a truly empty `event_journal`).
#[cfg(test)]
mod lifecycle_event_tests {
    use super::*;
    use crate::models::activity_dismissal::ActivityDismissal;
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

    async fn read_journal_rows(pool: &SqlitePool) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT event_type, payload FROM event_journal ORDER BY seq",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn hide_journal(pool: &SqlitePool) {
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(pool)
            .await
            .unwrap();
    }

    async fn unhide_journal(pool: &SqlitePool) {
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_emits_task_created() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();

        let task = Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        assert_eq!(rows.len(), 1, "exactly one event_journal row after create");
        assert_eq!(rows[0].0, "task_created");
        let event: NodeEvent = serde_json::from_str(&rows[0].1).unwrap();
        match event {
            NodeEvent::TaskCreated {
                task_id: tid,
                project_id: pid,
            } => {
                assert_eq!(tid, task.id);
                assert_eq!(pid, project_id);
            }
            other => panic!("expected TaskCreated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_status_emits_task_status_changed_with_both_statuses() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        Task::update_status(&pool, task_id, TaskStatus::InProgress)
            .await
            .unwrap();

        let rows = read_journal_rows(&pool).await;
        let status_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "task_status_changed")
            .collect();
        assert_eq!(status_rows.len(), 1, "exactly one task_status_changed row");
        let event: NodeEvent = serde_json::from_str(&status_rows[0].1).unwrap();
        match event {
            NodeEvent::TaskStatusChanged {
                task_id: tid,
                old_status,
                new_status,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(old_status, TaskStatus::Todo);
                assert_eq!(new_status, TaskStatus::InProgress);
            }
            other => panic!("expected TaskStatusChanged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_emits_task_deleted() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        let deleted = Task::delete(&pool, task_id).await.unwrap();
        assert_eq!(deleted, 1);

        let rows = read_journal_rows(&pool).await;
        let delete_rows: Vec<_> = rows.iter().filter(|(t, _)| t == "task_deleted").collect();
        assert_eq!(delete_rows.len(), 1, "exactly one task_deleted row");
        let event: NodeEvent = serde_json::from_str(&delete_rows[0].1).unwrap();
        match event {
            NodeEvent::TaskDeleted {
                task_id: tid,
                project_id: pid,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(pid, project_id);
            }
            other => panic!("expected TaskDeleted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_without_status_change_emits_no_status_event() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        let task = Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();
        assert_eq!(task.status, TaskStatus::Todo);

        Task::update(
            &pool,
            task_id,
            project_id,
            "new title".into(),
            task.description.clone(),
            TaskStatus::Todo, // unchanged
            task.parent_task_id,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        let status_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "task_status_changed")
            .collect();
        assert!(
            status_rows.is_empty(),
            "title-only update must not emit task_status_changed"
        );
    }

    /// Supplemental — NOT one of the task file's seven named tests. Test 4 proves Task::update
    /// does not emit when status is unchanged; without a companion positive-path test, a mutant
    /// that deletes the "only when differs" check entirely would still pass every named test
    /// (test 2 exercises the positive path via `update_status`, a different function). Added per
    /// the task 004 ledger lesson (2026-08-12): an all-green suite shipped a broken guard once
    /// already on this plan because the positive case for a conditional went untested.
    #[tokio::test]
    async fn update_with_status_change_emits_task_status_changed() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        let task = Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        Task::update(
            &pool,
            task_id,
            project_id,
            task.title.clone(),
            task.description.clone(),
            TaskStatus::InProgress,
            task.parent_task_id,
        )
        .await
        .unwrap();

        let rows = read_journal_rows(&pool).await;
        let status_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "task_status_changed")
            .collect();
        assert_eq!(
            status_rows.len(),
            1,
            "Task::update changing status must emit exactly one event"
        );
        let event: NodeEvent = serde_json::from_str(&status_rows[0].1).unwrap();
        match event {
            NodeEvent::TaskStatusChanged {
                task_id: tid,
                old_status,
                new_status,
            } => {
                assert_eq!(
                    tid, task_id,
                    "event must carry THIS task's id, not a default"
                );
                assert_eq!(old_status, TaskStatus::Todo);
                assert_eq!(new_status, TaskStatus::InProgress);
            }
            other => panic!("expected TaskStatusChanged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn failed_write_journals_nothing() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        // No project row exists for this id: the INSERT's FK constraint fails.
        let bogus_project_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();

        let result = Task::create(
            &pool,
            &CreateTask::from_title_description(bogus_project_id, "t".into(), None),
            task_id,
        )
        .await;
        assert!(result.is_err(), "FK violation must fail the write");

        let rows = read_journal_rows(&pool).await;
        assert!(
            rows.is_empty(),
            "failed write must journal nothing — proves the shared transaction"
        );
    }

    #[tokio::test]
    async fn delete_journals_inside_the_callers_transaction() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        // Rollback path: delete + its journal append must both vanish.
        {
            let mut tx = pool.begin().await.unwrap();
            Task::nullify_children_by_parent_id(&mut *tx, task_id)
                .await
                .unwrap();
            Task::delete(&mut *tx, task_id).await.unwrap();
            tx.rollback().await.unwrap();
        }
        let still_there = Task::find_by_id(&pool, task_id).await.unwrap();
        assert!(still_there.is_some(), "rollback must undo the delete");
        let rows = read_journal_rows(&pool).await;
        assert!(
            rows.iter().filter(|(t, _)| t == "task_deleted").count() == 0,
            "rollback must undo the journal append too"
        );

        // Commit path: both the delete and the journal append must land.
        {
            let mut tx = pool.begin().await.unwrap();
            Task::nullify_children_by_parent_id(&mut *tx, task_id)
                .await
                .unwrap();
            Task::delete(&mut *tx, task_id).await.unwrap();
            tx.commit().await.unwrap();
        }
        let gone = Task::find_by_id(&pool, task_id).await.unwrap();
        assert!(gone.is_none(), "commit must apply the delete");
        let rows = read_journal_rows(&pool).await;
        let delete_rows: Vec<_> = rows.iter().filter(|(t, _)| t == "task_deleted").collect();
        assert_eq!(
            delete_rows.len(),
            1,
            "commit must land exactly one task_deleted row"
        );
    }

    /// Required by the 2026-08-15 amendment to this task file: `Task::delete` has a SECOND
    /// production caller (`routes/tasks/handlers/remote.rs:254`) that passes the bare pool, not a
    /// caller-owned transaction. Proves that path is atomic too — `Acquire::begin` (not
    /// `Acquire::acquire`) opens a REAL transaction on a pool, so a failed journal append must roll
    /// back the DELETE. Fault injection follows the `crates/services` tailer tests' technique
    /// (`event_bus/mod.rs:750`): rename `event_journal` out from under the append so it fails with
    /// "no such table" — `chmod`/closing the pool do not inject a usable fault against sqlite's
    /// in-process driver.
    #[tokio::test]
    async fn delete_via_pool_is_atomic_when_append_fails() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        // Hide event_journal so the append inside Task::delete fails AFTER the DELETE would
        // otherwise have succeeded.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        // A2 guard (attempt-2 remediation, item 5): not just `is_err()` — pin WHICH statement
        // failed. Without this, a regression making `delete` fail at the SELECT/DELETE stage
        // instead of the append would keep this test green (it would still error, and the task
        // would trivially still be present), silently losing the property this test exists to
        // prove.
        let result = Task::delete(&pool, task_id).await;
        let err =
            result.expect_err("a failed journal append must surface as an error, not be swallowed");
        assert!(
            format!("{err:?}").contains("event_journal"),
            "the failure must be the journal append, not an earlier statement — otherwise the \
             assertions below pass vacuously with the DELETE never having run: {err:?}"
        );

        // Repair before asserting further, so the rest of THIS test isn't left with a renamed
        // table. (Not "the process-wide template database" — these tests use
        // create_test_pool_with_migrations, which builds a fresh TempDir per call
        // (test_utils.rs:108-129) and never touches the template `create_test_pool` copies from;
        // an earlier version of this comment claimed otherwise and was wrong.)
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        let still_there = Task::find_by_id(&pool, task_id).await.unwrap();
        assert!(
            still_there.is_some(),
            "Task::delete via the bare pool must be atomic: a failed journal append must not \
             leave the task deleted"
        );

        let rows = read_journal_rows(&pool).await;
        assert!(
            rows.iter().all(|(t, _)| t != "task_deleted"),
            "no task_deleted row should have landed when the append failed (the pre-existing \
             task_created row from Task::create is expected to still be there): {rows:?}"
        );
    }

    /// Covers the OTHER shape `Acquire::begin` produces: a `SAVEPOINT` nested inside a
    /// caller-owned transaction (`core.rs:663`'s call shape), not the top-level `BEGIN` the
    /// previous test exercises. Proves a failing append inside the savepoint does not poison the
    /// OUTER transaction — `tx.rollback()` on the caller's own transaction must still succeed
    /// cleanly afterward, which is the property `core.rs`'s atomicity with
    /// `nullify_children_by_parent_id` depends on.
    #[tokio::test]
    async fn delete_via_savepoint_rolls_back_cleanly_on_append_failure() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        // Hide event_journal OUTSIDE any transaction (a durable, committed rename) so it survives
        // the outer rollback below — SQLite DDL is transactional, so renaming it INSIDE the outer
        // tx would itself be undone by that same rollback, silently un-hiding the table before the
        // savepoint's append ever ran.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();

        let result = Task::delete(&mut *tx, task_id).await;
        assert!(
            result.is_err(),
            "a failed append inside the nested savepoint must surface as an error"
        );

        // The outer transaction must still be rollback-able cleanly: Task::delete's own
        // Transaction (the savepoint) was dropped without commit on the error path, which queues
        // a `ROLLBACK TO SAVEPOINT` (sqlx's Drop impl, transaction.rs:264-274) processed on this
        // same connection before this explicit rollback — it must not have left the connection in
        // a state where the caller's OWN rollback fails or hangs.
        tx.rollback().await.expect(
            "outer transaction must still roll back cleanly after the inner savepoint's failure",
        );

        // Repair (durable, outside any transaction) before asserting further.
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        let still_there = Task::find_by_id(&pool, task_id).await.unwrap();
        assert!(
            still_there.is_some(),
            "task must still exist after the outer transaction's rollback"
        );

        let rows = read_journal_rows(&pool).await;
        assert!(
            rows.iter().all(|(t, _)| t != "task_deleted"),
            "no task_deleted row should have landed: {rows:?}"
        );
    }

    /// Companion to `delete_via_savepoint_rolls_back_cleanly_on_append_failure`, which cannot
    /// detect a missing savepoint rollback: its final act is to roll the OUTER transaction back,
    /// so "the task still exists" is true whether or not the inner savepoint rolled anything back.
    /// Verified (attempt-2 review, item 5): that test passes UNCHANGED against a
    /// `.acquire()`-based `Task::delete`, i.e. against the exact defect it exists to disprove —
    /// panel 15B's finding, not this implementer's, and the task file's own specification error
    /// for requiring that assertion shape in the first place.
    ///
    /// This one COMMITS the outer transaction instead, which makes the assertion about the code
    /// under test rather than about the undo:
    ///  - the in-transaction read is the caller's very next command on the same connection, so it
    ///    is exactly what the FIFO-ordering claim predicts must already see `ROLLBACK TO
    ///    SAVEPOINT` applied (`Transaction::drop` -> `start_rollback` -> `Command::Rollback` on
    ///    the connection's worker channel, sqlx-core-0.8.6 transaction.rs:260-275);
    ///  - the outer commit then proves a failed append can never be made durable by a caller that
    ///    commits.
    #[tokio::test]
    async fn delete_savepoint_failure_is_undone_even_if_the_caller_commits() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        // Hide event_journal OUTSIDE any transaction: SQLite DDL is transactional, so renaming it
        // inside the outer tx would be undone by that tx and silently un-hide the table.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        let result = Task::delete(&mut *tx, task_id).await;
        assert!(
            result.is_err(),
            "a failed append inside the nested savepoint must surface as an error"
        );

        // The caller's NEXT command on this connection must already see the savepoint rollback.
        let visible_in_tx: Option<Uuid> =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM tasks WHERE id = ?")
                .bind(task_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap();

        // COMMIT, not rollback — this is what makes the test non-vacuous.
        let commit = tx.commit().await;

        // Repair (durable, outside any transaction) before asserting further.
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            visible_in_tx.is_some(),
            "the DELETE was still visible inside the outer transaction: the savepoint rollback \
             was NOT applied before the caller's next command on the same connection"
        );
        assert!(
            commit.is_ok(),
            "the outer transaction must still be committable after the inner savepoint failed: \
             {commit:?}"
        );
        let still_there = Task::find_by_id(&pool, task_id).await.unwrap();
        assert!(
            still_there.is_some(),
            "the caller's commit persisted a DELETE whose journal append had failed"
        );
        let rows = read_journal_rows(&pool).await;
        assert!(
            rows.iter().all(|(t, _)| t != "task_deleted"),
            "no task_deleted row may have landed: {rows:?}"
        );
    }

    /// The outer transaction must remain fully usable after a nested savepoint fails — a caller
    /// that logs the delete error and carries on must not have its own subsequent work silently
    /// dropped, nor have the failed delete smuggled into its commit.
    #[tokio::test]
    async fn failed_savepoint_leaves_the_outer_transaction_usable() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let doomed = Uuid::new_v4();
        let other = Uuid::new_v4();
        for id in [doomed, other] {
            Task::create(
                &pool,
                &CreateTask::from_title_description(project_id, "t".into(), None),
                id,
            )
            .await
            .unwrap();
        }

        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        assert!(Task::delete(&mut *tx, doomed).await.is_err());

        // Caller keeps using its transaction after the failure.
        let updated = sqlx::query("UPDATE tasks SET title = 'after' WHERE id = ?")
            .bind(other)
            .execute(&mut *tx)
            .await;
        let commit = tx.commit().await;

        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            updated.map(|r| r.rows_affected()).unwrap_or(0),
            1,
            "post-failure work on the outer transaction must still apply"
        );
        assert!(commit.is_ok(), "outer commit must succeed: {commit:?}");
        assert!(
            Task::find_by_id(&pool, doomed).await.unwrap().is_some(),
            "the failed delete must not have persisted through the caller's commit"
        );
        assert_eq!(
            Task::find_by_id(&pool, other).await.unwrap().unwrap().title,
            "after",
            "the caller's own post-failure work must have committed"
        );
    }

    #[tokio::test]
    async fn update_status_with_existing_dismissal_succeeds() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        ActivityDismissal::dismiss(&pool, task_id).await.unwrap();
        assert!(
            ActivityDismissal::is_dismissed(&pool, task_id)
                .await
                .unwrap()
        );

        Task::update_status(&pool, task_id, TaskStatus::InProgress)
            .await
            .expect("update_status with an existing dismissal must not deadlock");

        assert!(
            !ActivityDismissal::is_dismissed(&pool, task_id)
                .await
                .unwrap(),
            "dismissal must be cleared"
        );

        let rows = read_journal_rows(&pool).await;
        let status_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "task_status_changed")
            .collect();
        assert_eq!(status_rows.len(), 1, "exactly one task_status_changed row");
    }

    // ---------------------------------------------------------------------------------------
    // Panel 15A's six probes (attempt-2 remediation items 1, 2, 3). Sourced verbatim from
    // `/tmp/claude-1000/-data-Code-vk-swarm/7ada6c82-d888-446d-9d5c-48560bedfbbb/scratchpad/
    // panel15a-probes.rs.txt`, adapted only where it did not compile as-is against this module:
    // its own `seed_project`/`rows`/`hide`/`unhide` helpers are dropped in favour of this
    // module's `seed_project`/`read_journal_rows`/`hide_journal`/`unhide_journal`, and the
    // `mod panel15a_probes { ... }` wrapper is flattened into `lifecycle_event_tests` directly.
    // ---------------------------------------------------------------------------------------

    /// Item 1 (15A-1): the shipped suite never drove `update_status` with the SAME status. The
    /// guard this pins (`hierarchy.rs:62-64`, `old_status != status`) is load-bearing: seven
    /// production writers of Done/InReview call `update_status` without checking current status
    /// first (`git_ops.rs:99`, `github.rs:279`, `pr_monitor.rs:186,259`, `container.rs:296,597,
    /// 1594`), so a no-op call is a real, frequent path, not a corner case.
    #[tokio::test]
    async fn update_status_same_status_emits_no_status_event() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();

        // Task is already Todo. Writing Todo again is a no-op state change.
        Task::update_status(&pool, task_id, TaskStatus::Todo)
            .await
            .unwrap();

        let rows = read_journal_rows(&pool).await;
        let status_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "task_status_changed")
            .collect();
        assert!(
            status_rows.is_empty(),
            "no-op update_status must not emit task_status_changed: {status_rows:?}"
        );
    }

    /// Item 2 (15A-2), gap 1 of 3: append-failure atomicity was only proven for `delete`.
    /// `failed_write_journals_nothing` runs the opposite direction (a state-write failure), which
    /// is why this axis read as covered when it wasn't — an append failure with the state write
    /// SUCCEEDING (a committed task with no journal row) is the actual SC1 violation shape, and it
    /// survived every test in attempt 1.
    #[tokio::test]
    async fn create_rolls_back_when_append_fails() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        hide_journal(&pool).await;

        let result = Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await;
        assert!(result.is_err(), "create must fail when the append fails");

        unhide_journal(&pool).await;
        assert!(
            Task::find_by_id(&pool, task_id).await.unwrap().is_none(),
            "a failed append must roll back the task INSERT"
        );
    }

    /// Item 2, gap 2 of 3: same property for `update_status` — and pins the sub-gap the task file
    /// calls out specifically: the ACTIVITY DISMISSAL clear must ride the same transaction as the
    /// status write and the append, which is the entire reason `ActivityDismissal::clear_for_task`
    /// was generalized over `Executor` in the first place.
    #[tokio::test]
    async fn update_status_rolls_back_when_append_fails() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();
        ActivityDismissal::dismiss(&pool, task_id).await.unwrap();

        hide_journal(&pool).await;
        let result = Task::update_status(&pool, task_id, TaskStatus::InProgress).await;
        assert!(
            result.is_err(),
            "update_status must fail when the append fails"
        );
        unhide_journal(&pool).await;

        assert_eq!(
            Task::find_by_id(&pool, task_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Todo,
            "a failed append must roll back the status write"
        );
        assert!(
            ActivityDismissal::is_dismissed(&pool, task_id)
                .await
                .unwrap(),
            "a failed append must roll back the dismissal clear too"
        );
    }

    /// Item 2, gap 3 of 3: same property for `Task::update`.
    #[tokio::test]
    async fn update_rolls_back_when_append_fails() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        let task = Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "orig".into(), None),
            task_id,
        )
        .await
        .unwrap();

        hide_journal(&pool).await;
        let result = Task::update(
            &pool,
            task_id,
            project_id,
            "mutated".into(),
            task.description.clone(),
            TaskStatus::InProgress,
            task.parent_task_id,
        )
        .await;
        assert!(result.is_err(), "update must fail when the append fails");
        unhide_journal(&pool).await;

        let after = Task::find_by_id(&pool, task_id).await.unwrap().unwrap();
        assert_eq!(
            after.title, "orig",
            "a failed append must roll back the title write"
        );
        assert_eq!(after.status, TaskStatus::Todo, "and the status write");
    }

    /// Item 3 (15A-3), dedicated probe: `Task::update`'s event payload task_id, isolated from the
    /// rest of that test's assertions. (The shipped `update_with_status_change_emits_task_status_
    /// changed` test now also asserts this directly, per item 3's literal text — this probe is
    /// additional coverage handed down verified, not a replacement for that fix.)
    #[tokio::test]
    async fn update_event_carries_the_right_task_id() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let project_id = seed_project(&pool).await;
        let task_id = Uuid::new_v4();
        let task = Task::create(
            &pool,
            &CreateTask::from_title_description(project_id, "t".into(), None),
            task_id,
        )
        .await
        .unwrap();
        Task::update(
            &pool,
            task_id,
            project_id,
            task.title.clone(),
            task.description.clone(),
            TaskStatus::InProgress,
            task.parent_task_id,
        )
        .await
        .unwrap();
        let rows = read_journal_rows(&pool).await;
        let status_rows: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "task_status_changed")
            .collect();
        assert_eq!(status_rows.len(), 1);
        let event: NodeEvent = serde_json::from_str(&status_rows[0].1).unwrap();
        match event {
            NodeEvent::TaskStatusChanged { task_id: tid, .. } => assert_eq!(tid, task_id),
            other => panic!("expected TaskStatusChanged, got {other:?}"),
        }
    }

    /// GAP 6 from panel 15A's probe set: deleting a nonexistent id must not fabricate an event.
    /// Not cross-referenced by number in the task file's item list, but covered by its blanket
    /// "use all six probes" instruction.
    #[tokio::test]
    async fn delete_nonexistent_emits_nothing() {
        let (pool, _tmp) = create_test_pool_with_migrations().await;
        let deleted = Task::delete(&pool, Uuid::new_v4()).await.unwrap();
        assert_eq!(deleted, 0);
        assert!(
            read_journal_rows(&pool).await.is_empty(),
            "no-op delete must journal nothing"
        );
    }
}
