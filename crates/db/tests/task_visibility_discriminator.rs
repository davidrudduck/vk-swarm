//! Verifies the node-local visibility discriminator on
//! `Task::find_by_project_id_with_attempt_status`

use chrono::Utc;
use db::models::{
    project::{CreateProject, Project},
    task::{CreateTask, Task, TaskStatus},
};
use db::test_utils::create_test_pool;
use uuid::Uuid;

async fn make_project(pool: &sqlx::SqlitePool) -> Project {
    let id = Uuid::new_v4();
    let data = CreateProject {
        name: "Visibility Test".to_string(),
        git_repo_path: format!("/tmp/vis-{id}"),
        use_existing_repo: true,
        clone_url: None,
        setup_script: None,
        dev_script: None,
        cleanup_script: None,
        copy_files: None,
    };
    Project::create(pool, &data, id).await.expect("project")
}

async fn insert_local_attempt(pool: &sqlx::SqlitePool, task_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO task_attempts (id, task_id, executor, branch, target_branch)
           VALUES ($1, $2, 'CLAUDE_CODE', 'b', 'main')"#,
    )
    .bind(Uuid::new_v4())
    .bind(task_id)
    .execute(pool)
    .await
    .expect("attempt");
}

async fn insert_mirrored_remote_task(pool: &sqlx::SqlitePool, project_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO tasks (id, project_id, title, status, shared_task_id,
                              remote_version, remote_last_synced_at)
           VALUES ($1, $2, 'mirrored', 'todo', $3, 5, $4)"#,
    )
    .bind(id)
    .bind(project_id)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("mirrored insert");
    id
}

#[tokio::test]
async fn locally_created_task_is_visible() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let local = CreateTask::from_title_description(project.id, "local".into(), None);
    let local_id = Uuid::new_v4();
    Task::create(&pool, &local, local_id).await.expect("local task");

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(rows.iter().any(|r| r.task.id == local_id), "local task must be visible");
}

#[tokio::test]
async fn hive_assigned_task_with_local_attempt_is_visible() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let assigned = CreateTask::from_shared_task(
        project.id, "assigned".into(), None, TaskStatus::InProgress, Uuid::new_v4(),
    );
    let assigned_id = Uuid::new_v4();
    Task::create(&pool, &assigned, assigned_id).await.expect("assigned task");
    insert_local_attempt(&pool, assigned_id).await;

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(
        rows.iter().any(|r| r.task.id == assigned_id),
        "hive-assigned task WITH a local attempt must be visible"
    );
}

#[tokio::test]
async fn remote_mirrored_task_without_local_attempt_is_hidden() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let mirrored_id = insert_mirrored_remote_task(&pool, project.id).await;

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(
        !rows.iter().any(|r| r.task.id == mirrored_id),
        "remote-mirrored task with NO local attempt must NOT be visible"
    );
}

#[tokio::test]
async fn locally_created_then_shared_task_is_visible() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let task_id = Uuid::new_v4();
    let data = CreateTask::from_title_description(project.id, "shared-local".into(), None);
    Task::create(&pool, &data, task_id).await.expect("task");
    Task::set_shared_task_id(&pool, task_id, Some(Uuid::new_v4()))
        .await
        .expect("stamp shared_task_id");

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(
        rows.iter().any(|r| r.task.id == task_id),
        "locally-created-then-shared task (no attempt) must be visible"
    );
}
