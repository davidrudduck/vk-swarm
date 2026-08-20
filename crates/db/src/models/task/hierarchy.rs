//! Parent/child relationship operations for tasks.

use chrono::{DateTime, Utc};
use sqlx::{Executor, Sqlite, SqlitePool};
use uuid::Uuid;

use super::{Task, TaskRelationships, TaskStatus};
use crate::models::event::NodeEvent;
use crate::models::event_journal;
use crate::models::{activity_dismissal::ActivityDismissal, task_attempt::TaskAttempt};

impl Task {
    /// Update the status of a task and clear any activity dismissals.
    /// Also marks task for Hive resync by clearing remote_last_synced_at.
    ///
    /// Pool-taking site (task 006): opens its own transaction so the status write, the dismissal
    /// clear, and the `task_status_changed` journal append are all atomic together. Order inside
    /// the transaction is update status -> clear dismissal -> append event -> commit; the dismissal
    /// helper is called on the SAME transaction (not the pool) to avoid contending for SQLite's
    /// single writer lock on a second connection.
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: TaskStatus,
    ) -> Result<(), sqlx::Error> {
        // BEGIN IMMEDIATE takes SQLite's RESERVED (write) lock at BEGIN, so this transaction never
        // holds a read snapshot it must later upgrade — the SQLITE_BUSY_SNAPSHOT (517) class that a
        // deferred SELECT-first transaction earns is structurally gone, and contention surfaces at
        // BEGIN as plain SQLITE_BUSY, which busy_timeout retries (task 023).
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

        // Read the prior status INSIDE the transaction — see Task::update for why this can't
        // happen before begin() without racing a concurrent writer.
        let old_status: Option<TaskStatus> =
            sqlx::query_scalar::<_, TaskStatus>("SELECT status FROM tasks WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;

        sqlx::query!(
            "UPDATE tasks SET status = $2, updated_at = CURRENT_TIMESTAMP, activity_at = datetime('now', 'subsec'), remote_last_synced_at = NULL WHERE id = $1",
            id,
            status
        )
        .execute(&mut *tx)
        .await?;

        // Clear any activity dismissal when task status changes (auto-restore)
        ActivityDismissal::clear_for_task(&mut *tx, id).await?;

        // Emit ONLY when the status actually differs, mirroring Task::update's "exactly one event
        // per state change" rule.
        if let Some(old_status) = old_status
            && old_status != status
        {
            let event = NodeEvent::TaskStatusChanged {
                task_id: id,
                old_status,
                new_status: status,
            };
            event_journal::append(&mut *tx, &event)
                .await
                .map_err(sqlx::Error::from)?;
        }

        tx.commit().await?;

        Ok(())
    }

    /// Nullify parent_task_id for all tasks that reference the given parent task ID.
    /// This breaks parent-child relationships before deleting a parent task.
    pub async fn nullify_children_by_parent_id<'e, E>(
        executor: E,
        parent_id: Uuid,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!(
            "UPDATE tasks SET parent_task_id = NULL WHERE parent_task_id = $1",
            parent_id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected())
    }

    /// Find all child tasks for the given parent task ID.
    pub async fn find_children_by_parent_id(
        pool: &SqlitePool,
        parent_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // Find only child tasks that have this task as their parent
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
               WHERE parent_task_id = $1
               ORDER BY created_at DESC"#,
            parent_id,
        )
        .fetch_all(pool)
        .await
    }

    /// Find the relationships for a task attempt.
    /// Returns the parent task (if any), the current attempt, and child tasks.
    pub async fn find_relationships_for_attempt(
        pool: &SqlitePool,
        task_attempt: &TaskAttempt,
    ) -> Result<TaskRelationships, sqlx::Error> {
        // 1. Get the current task (task that owns this attempt)
        let current_task = Self::find_by_id(pool, task_attempt.task_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // 2. Get parent task (direct lookup via parent_task_id)
        let parent_task = if let Some(parent_id) = current_task.parent_task_id {
            Self::find_by_id(pool, parent_id).await?
        } else {
            None
        };

        // 3. Get children tasks (tasks that have this task as their parent)
        let children = Self::find_children_by_parent_id(pool, current_task.id).await?;

        Ok(TaskRelationships {
            parent_task,
            current_attempt: task_attempt.clone(),
            children,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        project::{CreateProject, Project},
        task::{CreateTask, tests::setup_test_pool},
    };

    #[tokio::test]
    async fn test_update_status() {
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
        let task_data =
            CreateTask::from_title_description(project_id, "Test Task".to_string(), None);
        let task = Task::create(&pool, &task_data, task_id)
            .await
            .expect("Failed to create task");
        assert_eq!(task.status, TaskStatus::Todo);

        // Update status
        Task::update_status(&pool, task_id, TaskStatus::InProgress)
            .await
            .expect("Update status failed");

        let updated = Task::find_by_id(&pool, task_id)
            .await
            .expect("Query failed")
            .expect("Task not found");
        assert_eq!(updated.status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_parent_child_relationships() {
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

        // Create parent task
        let parent_id = Uuid::new_v4();
        let parent_data =
            CreateTask::from_title_description(project_id, "Parent Task".to_string(), None);
        let _parent = Task::create(&pool, &parent_data, parent_id)
            .await
            .expect("Failed to create parent task");

        // Create child tasks
        for i in 0..3 {
            let child_id = Uuid::new_v4();
            let mut child_data =
                CreateTask::from_title_description(project_id, format!("Child Task {}", i), None);
            child_data.parent_task_id = Some(parent_id);
            Task::create(&pool, &child_data, child_id)
                .await
                .expect("Failed to create child task");
        }

        // Find children
        let children = Task::find_children_by_parent_id(&pool, parent_id)
            .await
            .expect("Query failed");
        assert_eq!(children.len(), 3);
        for child in &children {
            assert_eq!(child.parent_task_id, Some(parent_id));
        }

        // Nullify children
        let nullified = Task::nullify_children_by_parent_id(&pool, parent_id)
            .await
            .expect("Nullify failed");
        assert_eq!(nullified, 3);

        // Verify children no longer have parent
        let children = Task::find_children_by_parent_id(&pool, parent_id)
            .await
            .expect("Query failed");
        assert!(children.is_empty());
    }

    /// Regression guard for the SQLITE_BUSY_SNAPSHOT (517) class: if the transaction is ever
    /// reverted to a deferred `begin()` with a read as its first statement, concurrent status
    /// updates on the same row take a read snapshot and fail to upgrade. Only concurrency surfaces
    /// that. This test alternates between two statuses to exercise the status-changed path under
    /// contention.
    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_status_updates_serialize_without_errors() {
        let (pool, _temp_dir) = setup_test_pool().await;
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
        Project::create(&pool, &project_data, project_id)
            .await
            .unwrap();

        let task_id = Uuid::new_v4();
        let task_data =
            CreateTask::from_title_description(project_id, "Task Title".to_string(), None);
        Task::create(&pool, &task_data, task_id).await.unwrap();

        // 16 concurrent status updates of the SAME task, alternating between two statuses
        let mut handles = Vec::new();
        for version in 1..=16i64 {
            let pool = pool.clone();
            handles.push(tokio::spawn(async move {
                let status = if version % 2 == 0 {
                    TaskStatus::Todo
                } else {
                    TaskStatus::InProgress
                };
                Task::update_status(&pool, task_id, status).await
            }));
        }

        let mut errors: Vec<String> = Vec::new();
        for handle in handles {
            match handle.await.expect("update_status task panicked") {
                Ok(_) => {}
                Err(e) => errors.push(format!("{:?}", e)),
            }
        }
        assert!(
            errors.is_empty(),
            "all concurrent status updates must succeed, got {} error(s): {:?}",
            errors.len(),
            errors
        );

        // Read all task_status_changed events
        let status_change_payloads: Vec<(String,)> = sqlx::query_as(
            "SELECT payload FROM event_journal WHERE event_type = 'task_status_changed'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        // Every journaled row must record a REAL transition (old_status != new_status).
        // An old_status equal to new_status would mean the probe read a status that was not the
        // one the update overwrote — i.e. the probe escaped the write transaction.
        for (payload,) in &status_change_payloads {
            let event: NodeEvent =
                serde_json::from_str(payload).expect("failed to parse event payload");
            match event {
                NodeEvent::TaskStatusChanged {
                    old_status,
                    new_status,
                    ..
                } => {
                    assert_ne!(
                        old_status, new_status,
                        "every journaled status change must record a real transition"
                    );
                }
                other => panic!("expected TaskStatusChanged, got {other:?}"),
            }
        }
    }
}
