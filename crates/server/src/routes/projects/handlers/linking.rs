//! Remote project linking handlers.
//!
//! This module contains handlers for remote project operations:
//! - get_remote_project_by_id: Get remote project details
//! - get_project_remote_members: Get remote project members

use axum::{
    Extension,
    extract::{Path, State},
    response::Json as ResponseJson,
};
use db::models::project::Project;
use deployment::Deployment;
use utils::{
    api::projects::{RemoteProject, RemoteProjectMembersResponse},
    response::ApiResponse,
};
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn get_remote_project_by_id(
    State(deployment): State<DeploymentImpl>,
    Path(remote_project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<RemoteProject>>, ApiError> {
    let client = deployment.remote_client()?;

    let remote_project = client.get_project(remote_project_id).await?;

    Ok(ResponseJson(ApiResponse::success(remote_project)))
}

pub async fn get_project_remote_members(
    State(deployment): State<DeploymentImpl>,
    Extension(project): Extension<Project>,
) -> Result<ResponseJson<ApiResponse<RemoteProjectMembersResponse>>, ApiError> {
    let remote_project_id = project.remote_project_id.ok_or_else(|| {
        ApiError::Conflict("Project is not linked to a remote project".to_string())
    })?;

    let client = deployment.remote_client()?;

    let remote_project = client.get_project(remote_project_id).await?;
    let members = client
        .list_members(remote_project.organization_id)
        .await?
        .members;

    Ok(ResponseJson(ApiResponse::success(
        RemoteProjectMembersResponse {
            organization_id: remote_project.organization_id,
            members,
        },
    )))
}
