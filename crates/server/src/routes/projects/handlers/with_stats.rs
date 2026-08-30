//! Local projects with display enrichment: task counts, last attempt, and GitHub counts.
//!
//! The response deliberately carries no node/merge fields — this node serves local projects
//! only; hive-side project data lives on the hive (see ADR-0014).

use axum::{extract::State, response::Json as ResponseJson};
use db::models::project::Project;
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl,
    error::ApiError,
    routes::projects::types::{ProjectWithStats, ProjectsWithStatsResponse, TaskCounts},
};

/// List local projects with their display enrichment (task counts, last attempt, GitHub counts).
pub async fn get_projects_with_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ProjectsWithStatsResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let local_projects_with_stats = Project::find_local_projects_with_stats(pool).await?;

    let mut projects: Vec<ProjectWithStats> = local_projects_with_stats
        .into_iter()
        .map(|stats| {
            let project = stats.project;
            ProjectWithStats {
                id: project.id,
                name: project.name,
                git_repo_path: project.git_repo_path.to_string_lossy().to_string(),
                created_at: project.created_at,
                remote_project_id: project.remote_project_id,
                last_attempt_at: stats.last_attempt_at,
                github_enabled: project.github_enabled,
                github_owner: project.github_owner,
                github_repo: project.github_repo,
                github_open_issues: project.github_open_issues,
                github_open_prs: project.github_open_prs,
                github_last_synced_at: project.github_last_synced_at,
                task_counts: TaskCounts {
                    todo: stats.task_counts.todo,
                    in_progress: stats.task_counts.in_progress,
                    in_review: stats.task_counts.in_review,
                    done: stats.task_counts.done,
                },
            }
        })
        .collect();

    projects.sort_by_key(|p| p.name.to_lowercase());

    Ok(ResponseJson(ApiResponse::success(
        ProjectsWithStatsResponse { projects },
    )))
}
