use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension};
use lib::prelude::*;
use proto::scheduler_svc::DeleteTriggerRequest;

use crate::api::errors::ApiError;
use crate::api::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn delete(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let _ = scheduler
        .delete_trigger(DeleteTriggerRequest { name })
        .await?
        .into_inner();

    Ok(StatusCode::NO_CONTENT)
}
