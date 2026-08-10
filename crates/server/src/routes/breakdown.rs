//! Task breakdown lifecycle REST API: trigger, review, edit, and accept/discard/retry a
//! draft decomposition of a task into child tasks, plus a query for dependency edges.
//!
//! Two-stage trigger:
//! - STAGE 1 (`create_draft_proposal`) is synchronous and pool-only: the caller can rely
//!   on the returned proposal row existing at response time.
//! - STAGE 2 (`spawn_breakdown_run`) creates a task attempt (mirroring
//!   `create_task_and_start`'s attempt creation, `tasks/handlers/core.rs:305-452`) and starts
//!   the breakdown execution on it. It is fully awaitable -- it does NOT detach internally;
//!   the HTTP handler is responsible for detaching it via `tokio::spawn`.
//!
//! Any stage-2 error (attempt creation, spawn, linking) marks the proposal Failed with the
//! error text -- never leaves a stranded draft.

use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post, put},
};
use db::models::{
    project::Project,
    task::Task,
    task_attempt::{CreateTaskAttempt, TaskAttempt},
    task_breakdown::{
        self, BreakdownStatus, TaskBreakdownProposal, TaskBreakdownProposalItem, TaskDependency,
        UpsertProposalItems,
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{breakdown::BreakdownService, container::ContainerService};
use sqlx::SqlitePool;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// GET /tasks/{task_id}/breakdown response payload: the latest proposal (if any) and its items.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BreakdownWithItems {
    pub proposal: TaskBreakdownProposal,
    pub items: Vec<TaskBreakdownProposalItem>,
}

// ============================================================================
// Shared stage fns
// ============================================================================

/// STAGE 1: synchronous, pool-only. Rejects tasks of remote/mirrored projects using the
/// SAME guard `create_task_and_start` uses
/// (`crates/server/src/routes/tasks/handlers/core.rs:305-321`), then inserts a draft
/// proposal row. A concurrent draft for the same task maps the proposals table's
/// one-draft-per-task unique-index violation to `ApiError::Conflict` (409).
pub(crate) async fn create_draft_proposal(
    pool: &SqlitePool,
    task_id: Uuid,
) -> Result<TaskBreakdownProposal, ApiError> {
    let task = Task::find_by_id(pool, task_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    let project = Project::find_by_id(pool, task.project_id)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;

    if project.is_remote {
        return Err(ApiError::BadRequest(
            "Cannot start task attempts on remote projects. Tasks execute on their origin node."
                .to_string(),
        ));
    }

    task_breakdown::create(pool, task_id).await.map_err(|e| {
        if e.as_database_error()
            .is_some_and(|db_err| db_err.is_unique_violation())
        {
            ApiError::Conflict("A draft breakdown proposal already exists for this task".into())
        } else {
            ApiError::Database(e)
        }
    })
}

/// STAGE 2: creates a task attempt on the proposal's task and starts a breakdown execution
/// on it. AWAITABLE -- performs no internal detachment. On ANY error the proposal is marked
/// Failed (with the error text) before the error is returned, so a stranded draft never
/// results from a failed spawn.
pub(crate) async fn spawn_breakdown_run(
    deployment: DeploymentImpl,
    proposal: TaskBreakdownProposal,
) -> Result<(), ApiError> {
    match spawn_breakdown_run_inner(&deployment, &proposal).await {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = task_breakdown::update_status(
                &deployment.db().pool,
                proposal.id,
                BreakdownStatus::Failed,
                Some(err.to_string()),
            )
            .await;
            Err(err)
        }
    }
}

async fn spawn_breakdown_run_inner(
    deployment: &DeploymentImpl,
    proposal: &TaskBreakdownProposal,
) -> Result<(), ApiError> {
    let pool = &deployment.db().pool;

    let task = Task::find_by_id(pool, proposal.task_id)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;
    let project = Project::find_by_id(pool, task.project_id)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;

    let attempt_id = Uuid::new_v4();

    let origin_node_id = if let Some(ctx) = deployment.node_runner_context() {
        ctx.node_id().await
    } else {
        None
    };

    // Same base-branch resolution as an interactive attempt; also the point where an
    // invalid/missing project git repo surfaces as a stage-2 failure.
    let base_branch = deployment
        .git()
        .get_current_branch(&project.git_repo_path)?;
    let git_branch_name = deployment
        .container()
        .git_branch_from_task_attempt(&attempt_id, &task.title)
        .await;

    let executor_profile_id = deployment.config().read().await.executor_profile.clone();

    let task_attempt = TaskAttempt::create(
        pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor,
            base_branch,
            branch: git_branch_name,
            origin_node_id,
        },
        attempt_id,
        task.id,
    )
    .await?;

    let execution_process = deployment
        .container()
        .start_breakdown_attempt(
            &task_attempt,
            executor_profile_id,
            BreakdownService::breakdown_prompt(
                &task.title,
                task.description.as_deref().unwrap_or(""),
            ),
        )
        .await?;

    task_breakdown::link_execution_process(pool, proposal.id, execution_process.id).await?;

    Ok(())
}

/// Runs stage 1 (awaited) then detaches stage 2 via `tokio::spawn`. Used by the trigger
/// handler.
async fn trigger_and_spawn(
    deployment: DeploymentImpl,
    task_id: Uuid,
) -> Result<TaskBreakdownProposal, ApiError> {
    let proposal = create_draft_proposal(&deployment.db().pool, task_id).await?;
    let spawn_proposal = proposal.clone();
    tokio::spawn(async move {
        if let Err(err) = spawn_breakdown_run(deployment, spawn_proposal).await {
            tracing::error!(error = ?err, "breakdown run failed");
        }
    });
    Ok(proposal)
}

/// Pool-level body of the `get_breakdown` handler: 404 on unknown task, `None` when the
/// task has no proposal, otherwise the latest proposal with its items. The HTTP handler is
/// a thin State-unwrap wrapper over EXACTLY this fn so tests exercise the real code path.
pub(crate) async fn get_breakdown_impl(
    pool: &SqlitePool,
    task_id: Uuid,
) -> Result<Option<BreakdownWithItems>, ApiError> {
    Task::find_by_id(pool, task_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    let Some(proposal) = task_breakdown::find_by_task_id(pool, task_id).await? else {
        return Ok(None);
    };
    let items = task_breakdown::find_items(pool, proposal.id).await?;
    Ok(Some(BreakdownWithItems { proposal, items }))
}

/// Pool-level status gate + fresh-draft creation for the `retry` handler: 404 on unknown
/// proposal, 409 unless the proposal is Failed, then creates a fresh draft for the same
/// task. The HTTP handler calls EXACTLY this fn, then detaches the stage-2 spawn.
pub(crate) async fn retry_impl(
    pool: &SqlitePool,
    proposal_id: Uuid,
) -> Result<TaskBreakdownProposal, ApiError> {
    let proposal = task_breakdown::find_by_id(pool, proposal_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Breakdown proposal not found".to_string()))?;

    if proposal.status != BreakdownStatus::Failed {
        return Err(ApiError::Conflict(
            "Only a failed proposal can be retried".to_string(),
        ));
    }

    create_draft_proposal(pool, proposal.task_id).await
}

fn map_proposal_error(e: sqlx::Error) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound("Breakdown proposal not found".into()),
        sqlx::Error::Protocol(msg) => ApiError::Conflict(msg),
        other => ApiError::Database(other),
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /tasks/{task_id}/breakdown - Trigger a breakdown run for a task.
pub async fn trigger_breakdown(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TaskBreakdownProposal>>, ApiError> {
    let proposal = trigger_and_spawn(deployment, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(proposal)))
}

/// GET /tasks/{task_id}/breakdown - Latest proposal (+ items) for a task, or null if none.
pub async fn get_breakdown(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Option<BreakdownWithItems>>>, ApiError> {
    let result = get_breakdown_impl(&deployment.db().pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(result)))
}

/// PUT /breakdown-proposals/{id}/items - Replace a draft proposal's items.
pub async fn put_items(
    State(deployment): State<DeploymentImpl>,
    Path(proposal_id): Path<Uuid>,
    Json(payload): Json<UpsertProposalItems>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskBreakdownProposalItem>>>, ApiError> {
    let items = task_breakdown::replace_items(&deployment.db().pool, proposal_id, payload.items)
        .await
        .map_err(map_proposal_error)?;
    Ok(ResponseJson(ApiResponse::success(items)))
}

/// POST /breakdown-proposals/{id}/accept - Accept a draft proposal, creating child tasks.
pub async fn accept(
    State(deployment): State<DeploymentImpl>,
    Path(proposal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<Task>>>, ApiError> {
    let tasks = task_breakdown::accept_proposal(&deployment.db().pool, proposal_id)
        .await
        .map_err(map_proposal_error)?;
    Ok(ResponseJson(ApiResponse::success(tasks)))
}

/// POST /breakdown-proposals/{id}/discard - Discard a proposal.
pub async fn discard(
    State(deployment): State<DeploymentImpl>,
    Path(proposal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TaskBreakdownProposal>>, ApiError> {
    let proposal = task_breakdown::update_status(
        &deployment.db().pool,
        proposal_id,
        BreakdownStatus::Discarded,
        None,
    )
    .await
    .map_err(map_proposal_error)?;
    Ok(ResponseJson(ApiResponse::success(proposal)))
}

/// POST /breakdown-proposals/{id}/retry - Retry a failed proposal: creates a fresh draft
/// (and a fresh run) for the same task.
pub async fn retry(
    State(deployment): State<DeploymentImpl>,
    Path(proposal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<TaskBreakdownProposal>>, ApiError> {
    let new_proposal = retry_impl(&deployment.db().pool, proposal_id).await?;
    let spawn_proposal = new_proposal.clone();
    tokio::spawn(async move {
        if let Err(err) = spawn_breakdown_run(deployment, spawn_proposal).await {
            tracing::error!(error = ?err, "breakdown run failed");
        }
    });
    Ok(ResponseJson(ApiResponse::success(new_proposal)))
}

/// GET /tasks/{task_id}/dependencies - Dependency edges where `task_id` depends on others.
pub async fn get_dependencies(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskDependency>>>, ApiError> {
    let deps = task_breakdown::find_dependencies(&deployment.db().pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(deps)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/tasks/{task_id}/breakdown",
            get(get_breakdown).post(trigger_breakdown),
        )
        .route("/tasks/{task_id}/dependencies", get(get_dependencies))
        .route("/breakdown-proposals/{id}/items", put(put_items))
        .route("/breakdown-proposals/{id}/accept", post(accept))
        .route("/breakdown-proposals/{id}/discard", post(discard))
        .route("/breakdown-proposals/{id}/retry", post(retry))
}

#[cfg(test)]
mod tests {
    use db::models::{
        project::CreateProject,
        task::CreateTask,
        task_breakdown::{ProposalItemInput, TaskBreakdownProposalItem},
    };
    use db::test_utils::create_test_pool;

    use super::*;

    async fn seed_task(pool: &SqlitePool, is_remote: bool) -> (Project, Task) {
        let project = Project::create(
            pool,
            &CreateProject {
                name: "proj".to_string(),
                git_repo_path: "/tmp/breakdown-test-repo".to_string(),
                use_existing_repo: true,
                clone_url: None,
                setup_script: None,
                dev_script: None,
                cleanup_script: None,
                copy_files: None,
            },
            Uuid::new_v4(),
        )
        .await
        .unwrap();

        if is_remote {
            sqlx::query("UPDATE projects SET is_remote = 1, source_node_id = ? WHERE id = ?")
                .bind(Uuid::new_v4())
                .bind(project.id)
                .execute(pool)
                .await
                .unwrap();
        }
        let project = Project::find_by_id(pool, project.id)
            .await
            .unwrap()
            .unwrap();

        let task = Task::create(
            pool,
            &CreateTask {
                project_id: project.id,
                title: "goal task".to_string(),
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

        (project, task)
    }

    fn item(title: &str, sort_order: i64, depends_on: Vec<i64>) -> ProposalItemInput {
        ProposalItemInput {
            title: title.to_string(),
            description: None,
            sort_order,
            depends_on_indices: depends_on,
        }
    }

    #[tokio::test]
    async fn test_trigger_creates_draft_and_409_on_second() {
        let (pool, _tmp) = create_test_pool().await;
        let (_project, task) = seed_task(&pool, false).await;

        let proposal = create_draft_proposal(&pool, task.id).await.unwrap();
        assert_eq!(proposal.status, BreakdownStatus::Draft);
        assert_eq!(proposal.task_id, task.id);

        let second = create_draft_proposal(&pool, task.id).await;
        assert!(matches!(second, Err(ApiError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_review_gate_no_outbox_before_accept() {
        let (pool, _tmp) = create_test_pool().await;
        let (_project, task) = seed_task(&pool, false).await;

        let outbox_before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM node_outbox WHERE entity_type = 'task'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let proposal = create_draft_proposal(&pool, task.id).await.unwrap();
        let items = task_breakdown::replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0])],
        )
        .await
        .unwrap();

        let outbox_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM node_outbox WHERE entity_type = 'task'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            outbox_before, outbox_after,
            "no outbox rows should be enqueued before accept"
        );

        // Zero node_outbox rows whose entity_id equals the proposal id or any item id.
        let mut ids = vec![proposal.id];
        ids.extend(items.iter().map(|i: &TaskBreakdownProposalItem| i.id));
        for id in ids {
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM node_outbox WHERE entity_id = ?")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(count, 0, "unexpected outbox row for id {id}");
        }
    }

    #[tokio::test]
    async fn test_accept_returns_children_and_edges() {
        let (pool, _tmp) = create_test_pool().await;
        let (_project, task) = seed_task(&pool, false).await;

        let proposal = create_draft_proposal(&pool, task.id).await.unwrap();
        task_breakdown::replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0])],
        )
        .await
        .unwrap();

        let created = task_breakdown::accept_proposal(&pool, proposal.id)
            .await
            .unwrap();
        assert_eq!(created.len(), 2);

        let task_b = created.iter().find(|t| t.title == "B").unwrap();
        let deps = task_breakdown::find_dependencies(&pool, task_b.id)
            .await
            .unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].task_id, task_b.id);

        let task_a = created.iter().find(|t| t.title == "A").unwrap();
        assert_eq!(deps[0].depends_on_task_id, task_a.id);
    }

    #[tokio::test]
    async fn test_edit_items_only_in_draft() {
        let (pool, _tmp) = create_test_pool().await;
        let (_project, task) = seed_task(&pool, false).await;

        let proposal = create_draft_proposal(&pool, task.id).await.unwrap();
        task_breakdown::replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![])],
        )
        .await
        .unwrap();
        task_breakdown::accept_proposal(&pool, proposal.id)
            .await
            .unwrap();

        let result = task_breakdown::replace_items(&pool, proposal.id, vec![item("C", 0, vec![])])
            .await
            .map_err(map_proposal_error);
        assert!(matches!(result, Err(ApiError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_discard_and_retry() {
        let (pool, _tmp) = create_test_pool().await;
        let (_project, task) = seed_task(&pool, false).await;

        let proposal = create_draft_proposal(&pool, task.id).await.unwrap();
        let discarded =
            task_breakdown::update_status(&pool, proposal.id, BreakdownStatus::Discarded, None)
                .await
                .unwrap();
        assert_eq!(discarded.status, BreakdownStatus::Discarded);

        // Discarded frees the one-draft-per-task slot: a new trigger succeeds.
        let retried_ok = create_draft_proposal(&pool, task.id).await;
        assert!(retried_ok.is_ok());

        // Retry-after-failure path: mark that new draft Failed, then retry creates a fresh draft.
        let failed_proposal = retried_ok.unwrap();
        let failed = task_breakdown::update_status(
            &pool,
            failed_proposal.id,
            BreakdownStatus::Failed,
            Some("boom".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(failed.status, BreakdownStatus::Failed);

        let fresh_draft = create_draft_proposal(&pool, task.id).await.unwrap();
        assert_eq!(fresh_draft.status, BreakdownStatus::Draft);
        assert_ne!(fresh_draft.id, failed.id);
    }

    /// BOUNDARY (ledgered): this is the only test that spins up a real `LocalDeployment`
    /// (env-var-isolated into a tempdir, mirroring `crates/server/tests/common::hive_absent`)
    /// because `spawn_breakdown_run` needs `deployment.git()`/`deployment.container()`, which
    /// are `impl Trait` returns (not `dyn`-mockable) on the `Deployment` trait. It does NOT
    /// spawn a real CLI/executor: the project's git_repo_path is intentionally invalid, so
    /// `get_current_branch` fails before any executor would be started, and that failure is
    /// what exercises the Failed-marking path.
    #[tokio::test]
    #[serial_test::serial]
    async fn test_spawn_failure_marks_failed() {
        unsafe {
            std::env::remove_var("VK_HIVE_URL");
            std::env::remove_var("VK_NODE_API_KEY");
            std::env::remove_var("VK_SHARED_API_BASE");
            std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1");
            std::env::set_var("DISABLE_WORKTREE_EXPIRED_CLEANUP", "1");
        }
        let temp_dir = tempfile::TempDir::new().unwrap();
        unsafe {
            std::env::set_var("VK_ASSET_DIR", temp_dir.path());
            std::env::set_var("VK_DATABASE_PATH", temp_dir.path().join("db.sqlite"));
        }

        let deployment = local_deployment::LocalDeployment::new().await.unwrap();
        let pool = deployment.db().pool.clone();

        let (_project, task) = seed_task(&pool, false).await;
        let proposal = create_draft_proposal(&pool, task.id).await.unwrap();

        let result = spawn_breakdown_run(deployment.clone(), proposal.clone()).await;
        assert!(result.is_err());

        let failed = task_breakdown::find_by_id(&pool, proposal.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.status, BreakdownStatus::Failed);
        assert!(failed.error.is_some());
        assert!(failed.execution_process_id.is_none());

        // A subsequent trigger succeeds with a new draft (no stranded draft left behind).
        let retried = create_draft_proposal(&pool, task.id).await.unwrap();
        assert_eq!(retried.status, BreakdownStatus::Draft);
        assert_ne!(retried.id, proposal.id);
    }

    #[tokio::test]
    async fn test_remote_project_rejected() {
        let (pool, _tmp) = create_test_pool().await;
        let (project, task) = seed_task(&pool, true).await;
        assert!(project.is_remote);

        let before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM task_breakdown_proposals WHERE task_id = ?")
                .bind(task.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(before, 0);

        let result = create_draft_proposal(&pool, task.id).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));

        let after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM task_breakdown_proposals WHERE task_id = ?")
                .bind(task.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after, 0, "no proposal row should be created");
    }

    #[tokio::test]
    async fn test_get_breakdown_unknown_task_404() {
        let (pool, _tmp) = create_test_pool().await;

        let unknown_task_id = Uuid::new_v4();
        let result = get_breakdown_impl(&pool, unknown_task_id).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));

        let (_project, task) = seed_task(&pool, false).await;
        let ok = get_breakdown_impl(&pool, task.id).await.unwrap();
        assert!(
            ok.is_none(),
            "existing task with no proposal returns data: null"
        );
    }

    #[tokio::test]
    async fn test_retry_gate() {
        let (pool, _tmp) = create_test_pool().await;
        let (_project, task) = seed_task(&pool, false).await;

        // Unknown proposal -> 404.
        let missing = retry_impl(&pool, Uuid::new_v4()).await;
        assert!(matches!(missing, Err(ApiError::NotFound(_))));

        // Draft proposal -> 409 Conflict (Failed-only gate).
        let draft = create_draft_proposal(&pool, task.id).await.unwrap();
        let on_draft = retry_impl(&pool, draft.id).await;
        assert!(matches!(on_draft, Err(ApiError::Conflict(_))));

        // Accepted proposal -> 409 Conflict too.
        task_breakdown::replace_items(&pool, draft.id, vec![item("A", 0, vec![])])
            .await
            .unwrap();
        task_breakdown::accept_proposal(&pool, draft.id)
            .await
            .unwrap();
        let on_accepted = retry_impl(&pool, draft.id).await;
        assert!(matches!(on_accepted, Err(ApiError::Conflict(_))));

        // Failed proposal -> Ok fresh draft for the same task.
        let failed_src = create_draft_proposal(&pool, task.id).await.unwrap();
        let failed = task_breakdown::update_status(
            &pool,
            failed_src.id,
            BreakdownStatus::Failed,
            Some("boom".to_string()),
        )
        .await
        .unwrap();

        let fresh = retry_impl(&pool, failed.id).await.unwrap();
        assert_eq!(fresh.status, BreakdownStatus::Draft);
        assert_eq!(fresh.task_id, task.id);
        assert_ne!(fresh.id, failed.id);
    }
}
