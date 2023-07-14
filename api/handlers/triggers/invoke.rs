use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{Invocation, ProjectId, RequestId, TriggerId};
use proto::scheduler_proto::InvokeTriggerRequest;

use crate::api_model::InvokeTrigger;
use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn invoke(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
    Extension(request_id): Extension<RequestId>,
    ValidatedId(id): ValidatedId<TriggerId>,
    // The body of the request is optional, so we use Option<Json<...>>.
    request: Option<Json<InvokeTrigger>>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(request) = request.unwrap_or_default();
    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    // Send the request to the scheduler
    let invoke_request = InvokeTriggerRequest {
        project_id: project.into(),
        id: id.into(),
        mode: request.mode.into(),
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
