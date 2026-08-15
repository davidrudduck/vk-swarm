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

    /// Delete a task and journal `TaskDeleted` on the SAME executor — no owned transaction, no
    /// commit. The caller (`crates/server/src/routes/tasks/handlers/core.rs`) already owns a
    /// transaction spanning `nullify_children_by_parent_id` + this delete; committing here would
    /// break that atomicity, and a nested `begin()` on a generic consumed executor isn't
    /// expressible anyway.
    ///
    /// Bound is `Acquire`, not bare `Executor`: the delete needs THREE sequential statements
    /// (read project_id for the event payload, DELETE, append) against the one executor it was
    /// given, and a bare `E: Executor` value is consumed by each call with no way to reborrow an
    /// opaque generic. `Acquire::acquire()` on `&mut SqliteConnection` is a no-op passthrough (NOT
    /// `.begin()` — no transaction opened) that hands back a concrete `&mut SqliteConnection`,
    /// which CAN be reborrowed via `&mut *conn` for each statement. Every existing caller already
    /// passes `&SqlitePool` or `&mut *tx` (i.e. `&mut SqliteConnection`), both of which implement
    /// `Acquire`, so no call site needs to change.
    // NOT `async fn`: collapsing this to `async fn delete<'e, E>(...) -> Result<u64, sqlx::Error>
    // where E: Acquire<'e, ...>` (clippy's own suggestion) reintroduces "implementation of
    // `sqlx::Acquire` is not general enough" at the real caller
    // (`routes/tasks/handlers/core.rs`'s `Task::delete(&mut *tx, task.id)` inside the axum
    // handler) — a documented sqlx/rustc HRTB limitation when an `async fn`'s single elided
    // lifetime is forced to serve both the `Acquire` bound and the returned future's own capture
    // lifetime. Decoupling them into separate `'a` (the future) and `'c` (the `Acquire` bound) via
    // a hand-written `async move` block is sqlx's own documented workaround; see the `Acquire`
    // trait's doc comment in sqlx-core for the same two-lifetime pattern.
    #[allow(clippy::manual_async_fn)]
    pub fn delete<'a, 'c, E>(
        executor: E,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<u64, sqlx::Error>> + Send + 'a
    where
        E: Acquire<'c, Database = Sqlite> + Send + 'a,
    {
        async move {
            // `begin`, not `acquire`: two production callers exist
            // (`routes/tasks/handlers/core.rs:663` passes `&mut *tx`,
            // `routes/tasks/handlers/remote.rs:254` passes the bare pool) and only `begin` makes
            // BOTH atomic. On `&mut Transaction` it is a `SAVEPOINT` nested in the caller's
            // transaction (a pure passthrough would have been `acquire`, sqlx-core-0.8.6
            // transaction.rs:250) — its own `commit()` below is a `RELEASE SAVEPOINT`, not a real
            // commit, so the caller's outer `tx.commit()` is still what makes it durable. On a
            // bare pool it opens a REAL transaction, closing the gap where three separate
            // autocommit statements could leave a deleted task with no journal row.
            let mut tx = executor.begin().await?;

            // Read identity for the event payload BEFORE the delete, on the same executor.
            let project_id: Option<Uuid> =
                sqlx::query_scalar::<_, Uuid>("SELECT project_id FROM tasks WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;

            let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
                .execute(&mut *tx)
                .await?;

            // Only journal a real deletion — a delete of a nonexistent id must not fabricate an
            // event.
            if result.rows_affected() > 0
                && let Some(project_id) = project_id
            {
                let event = NodeEvent::TaskDeleted {
                    task_id: id,
                    project_id,
                };
                event_journal::append(&mut *tx, &event)
                    .await
                    .map_err(journal_err_to_sqlx)?;
            }

            tx.commit().await?;

            Ok(result.rows_affected())
        }
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
                old_status,
                new_status,
                ..
            } => {
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

        let result = Task::delete(&pool, task_id).await;
        assert!(
            result.is_err(),
            "a failed journal append must surface as an error, not be swallowed"
        );

        // Repair before asserting further, so the rest of this test (and the process-wide template
        // database other tests copy from) isn't left with a renamed table.
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
}
