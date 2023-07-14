use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{Invocation, OwnerId, TriggerId, ValidId};
use proto::scheduler_proto::InvokeTriggerRequest;

use crate::errors::ApiError;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn invoke(
    state: State<Arc<AppState>>,
    Extension(owner_id): Extension<OwnerId>,
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
            Json("Invalid trigger id"),
        )
            .into_response());
    }

    let mut scheduler = state.scheduler_for_trigger(&id).await?;
    // Send the request to the scheduler
    let invoke_request = InvokeTriggerRequest {
        owner_id: owner_id.0.clone(),
        id: id.into(),
    };
    let invocation = scheduler
        .invoke_trigger(invoke_request)
        .await?
        .into_inner()
        .invocation
        .unwrap();
    let invocation: Invocation = invocation.into();

    Ok((StatusCode::CREATED, response_headers, Json(invocation))
        .into_response())
}
