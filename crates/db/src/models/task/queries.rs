//! CRUD query operations for tasks.

use chrono::{DateTime, Utc};
use sqlx::{Executor, Sqlite, SqlitePool};
use uuid::Uuid;

use super::{CreateTask, Task, TaskStatus, TaskWithAttemptStatus};
use crate::models::project::Project;

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
        .fetch_one(pool)
        .await?;
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
        .fetch_one(pool)
        .await?;
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

    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
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
    pub async fn find_digest_entries(
        pool: &SqlitePool,
    ) -> Result<Vec<TaskDigestRow>, sqlx::Error> {
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
                .map(|r| TaskDigestRow { id: r.id, remote_version: r.remote_version })
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
            entries[0].remote_version,
            3,
            "version is the task's remote_version"
        );
        assert_eq!(entries[0].id, linked_id, "entity_id == the linked task's LOCAL id");
    }
}
