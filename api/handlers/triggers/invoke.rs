use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{Invocation, ProjectId, RequestId, TriggerId, ValidId};
use proto::scheduler_proto::InvokeTriggerRequest;

use crate::errors::ApiError;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn invoke(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
    Extension(request_id): Extension<RequestId>,
    Path(id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    if !id.is_valid() {
        return Err(ApiError::NotFound(id.to_string()));
    }

    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    // Send the request to the scheduler
    let invoke_request = InvokeTriggerRequest {
        project_id: project.0.clone(),
        id: id.into(),
    };
    let invocation = scheduler
        .invoke_trigger(invoke_request)
        .await?
        .into_inner()
        .invocation
        .unwrap();
    let invocation: Invocation = invocation.into();

    Ok((StatusCode::CREATED, Json(invocation)).into_response())
}
