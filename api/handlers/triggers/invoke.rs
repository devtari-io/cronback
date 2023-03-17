use std::sync::Arc;

use axum::{
    debug_handler,
    extract::{Path, State},
    http::{header::HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use proto::scheduler_proto::InvokeTriggerRequest;

use crate::{api_model::InvokeTrigger, errors::ApiError, AppState};
use shared::types::{Invocation, TriggerId, ValidId};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn invoke(
    state: State<Arc<AppState>>,
    Path(id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    if !id.is_valid() {
        return Ok((
            StatusCode::BAD_REQUEST,
            response_headers,
            // TODO: We need a proper API design for API errors
            Json(Err("Invalid trigger id")),
        ));
    }

    let mut scheduler = state.scheduler_for_trigger(&id).await?;
    // Send the request to the scheduler
    let request = InvokeTrigger::from_id(id);

    let invoke_request: InvokeTriggerRequest = request.into();
    let invocation = scheduler
        .invoke_trigger(invoke_request)
        .await?
        .into_inner()
        .invocation
        .unwrap();
    let invocation: Invocation = invocation.into();

    Ok((StatusCode::CREATED, response_headers, Json(Ok(invocation))))
}
