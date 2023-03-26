use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use proto::scheduler_proto::InvokeTriggerRequest;
use shared::types::{Invocation, OwnerId, TriggerId, ValidId};

use crate::api_model::InvokeTrigger;
use crate::errors::ApiError;
use crate::{AppState, AppStateError};

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

    let Some(trigger) = state
        .db
        .trigger_store
        .get_trigger(&id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))? else {
            return Ok(
                StatusCode::NOT_FOUND.into_response()
            );
        };

    if trigger.owner_id != owner_id {
        return Ok(StatusCode::FORBIDDEN.into_response());
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

    Ok((StatusCode::CREATED, response_headers, Json(invocation))
        .into_response())
}
