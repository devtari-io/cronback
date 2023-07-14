use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{ProjectId, RequestId, TriggerId, TriggerManifest, ValidId};
use proto::scheduler_proto::PauseTriggerRequest;

use crate::errors::ApiError;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn pause(
    state: State<Arc<AppState>>,
    Path(id): Path<TriggerId>,
    Extension(project): Extension<ProjectId>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    if !id.is_valid() {
        return Err(ApiError::NotFound(id.to_string()));
    }

    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    let trigger = scheduler
        .pause_trigger(PauseTriggerRequest {
            project_id: project.0.clone(),
            id: id.0,
        })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: TriggerManifest = trigger.into();

    Ok((StatusCode::OK, Json(trigger)).into_response())
}
