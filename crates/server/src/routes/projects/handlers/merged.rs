//! Merged view handlers for projects.
//!
//! Local-only implementation: the node UI's project list is served entirely
//! from local state. Remote node locations are no longer merged in at request
//! time (the hive is displayed via the hive-sync view instead), so `nodes` is
//! always empty and `has_local` is always true.

use axum::{extract::State, response::Json as ResponseJson};
use db::models::project::Project;
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

use super::super::types::{MergedProject, MergedProjectsResponse, TaskCounts};

/// Get the project list for the node UI, shaped as `MergedProjectsResponse`.
///
/// Every entry is a local project (`has_local: true`, `nodes: []`). Linked
/// projects still carry their `remote_project_id` so the UI can show sync
/// status.
pub async fn get_merged_projects(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MergedProjectsResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let local_projects_with_stats = Project::find_local_projects_with_stats(pool).await?;

    let mut projects: Vec<MergedProject> = local_projects_with_stats
        .into_iter()
        .map(|stats| {
            let project = stats.project;
            MergedProject {
                id: project.id,
                name: project.name,
                git_repo_path: project.git_repo_path.to_string_lossy().to_string(),
                created_at: project.created_at,
                remote_project_id: project.remote_project_id,
                has_local: true,
                local_project_id: Some(project.id),
                nodes: Vec::new(),
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

    projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(ResponseJson(ApiResponse::success(MergedProjectsResponse {
        projects,
    })))
}
