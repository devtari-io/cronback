use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use lib::model::ValidShardedId;
use lib::types::ProjectId;
use serde::Serialize;

use crate::errors::ApiError;
use crate::AppState;

#[derive(Serialize, Debug)]
struct CreateProjectResponse {
    project: ValidShardedId<ProjectId>,
}

#[tracing::instrument(skip(_state))]
pub(crate) async fn create(
    _state: State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let response = CreateProjectResponse {
        project: ProjectId::generate(),
    };
    Ok(Json(response))
}
