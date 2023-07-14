use axum::response::IntoResponse;
use axum::Json;
use lib::model::ValidShardedId;
use lib::types::ProjectId;
use serde::Serialize;

use crate::errors::ApiError;

#[derive(Serialize, Debug)]
struct CreateProjectResponse {
    project: ValidShardedId<ProjectId>,
}

#[tracing::instrument]
pub(crate) async fn create() -> Result<impl IntoResponse, ApiError> {
    let response = CreateProjectResponse {
        project: ProjectId::generate(),
    };
    Ok(Json(response))
}
