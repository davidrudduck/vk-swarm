//! Workstream state: read-only assembling view over task_attempts + execution_processes + executor_sessions.
//!
//! This view projects the "run-state triple" (attempt + process + session) for recovery and
//! downstream query. No new run entity; this is a projection of existing tables (SC3).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct WorkstreamState {
    pub execution_process_id: Uuid,
    pub task_attempt_id: Uuid,
    pub container_ref: Option<String>,
    pub branch: String,
    pub target_branch: String,
    pub run_reason: Option<String>,
    pub status: String,
    pub resume_state: Option<String>,
    pub pid: Option<i64>,
    pub before_head_commit: Option<String>,
    pub after_head_commit: Option<String>,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WorkstreamState {
    /// Find all workstream states for a task attempt.
    pub async fn find_by_task_attempt(
        pool: &SqlitePool,
        attempt_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkstreamState,
            r#"SELECT
                execution_process_id as "execution_process_id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                container_ref,
                branch,
                target_branch,
                run_reason,
                status,
                resume_state,
                pid,
                before_head_commit,
                after_head_commit,
                session_id,
                created_at as "created_at!: DateTime<Utc>"
               FROM v_workstream_state
               WHERE task_attempt_id = ?
               ORDER BY created_at DESC"#,
            attempt_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find resumable running workstream states.
    pub async fn find_resumable_running(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkstreamState,
            r#"SELECT
                execution_process_id as "execution_process_id!: Uuid",
                task_attempt_id as "task_attempt_id!: Uuid",
                container_ref,
                branch,
                target_branch,
                run_reason,
                status,
                resume_state,
                pid,
                before_head_commit,
                after_head_commit,
                session_id,
                created_at as "created_at!: DateTime<Utc>"
               FROM v_workstream_state
               WHERE status = 'running'
               ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seed_running_attempt_with_session(
        pool: &SqlitePool,
        session_id: &str,
    ) -> Uuid {
        let now = Utc::now();
        let project_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let process_id = Uuid::new_v4();
        let session_uuid = Uuid::new_v4();

        // Insert project
        sqlx::query(
            r#"INSERT INTO projects (id, name, git_repo_path, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(project_id.as_bytes().to_vec())
        .bind("test-project")
        .bind("/tmp/test-repo")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert project");

        // Insert task
        sqlx::query(
            r#"INSERT INTO tasks (id, project_id, title, status, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(task_id.as_bytes().to_vec())
        .bind(project_id.as_bytes().to_vec())
        .bind("test-task")
        .bind("inprogress")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert task");

        // Insert task_attempt
        sqlx::query(
            r#"INSERT INTO task_attempts
               (id, task_id, container_ref, branch, target_branch, executor, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(attempt_id.as_bytes().to_vec())
        .bind(task_id.as_bytes().to_vec())
        .bind("/tmp/test-container")
        .bind("test-branch")
        .bind("main")
        .bind("test-executor")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert task_attempt");

        // Insert execution_process
        sqlx::query(
            r#"INSERT INTO execution_processes
               (id, task_attempt_id, run_reason, status, executor_action, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(process_id.as_bytes().to_vec())
        .bind(attempt_id.as_bytes().to_vec())
        .bind("codingagent")
        .bind("running")
        .bind("{}")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert execution_process");

        // Insert executor_session
        sqlx::query(
            r#"INSERT INTO executor_sessions
               (id, task_attempt_id, execution_process_id, session_id, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(session_uuid.as_bytes().to_vec())
        .bind(attempt_id.as_bytes().to_vec())
        .bind(process_id.as_bytes().to_vec())
        .bind(session_id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert executor_session");

        attempt_id
    }

    #[tokio::test]
    async fn test_workstream_state_assembles_the_triple() {
        let (pool, _tmp) = crate::test_utils::create_test_pool().await;
        let attempt_id = seed_running_attempt_with_session(&pool, "sess-123").await;
        let rows = WorkstreamState::find_by_task_attempt(&pool, attempt_id)
            .await
            .unwrap();
        let row = rows.first().expect("one assembled row");
        assert_eq!(row.session_id.as_deref(), Some("sess-123"));
        assert!(row.container_ref.is_some());
        assert_eq!(row.status, "running");
        assert!(row.resume_state.is_none());
    }
}
