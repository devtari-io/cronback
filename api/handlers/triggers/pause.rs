use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{OwnerId, Trigger, TriggerId, ValidId};
use proto::scheduler_proto::PauseTriggerRequest;
use tracing::info;

use crate::errors::ApiError;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn pause(
    state: State<Arc<AppState>>,
    Path(id): Path<TriggerId>,
    Extension(owner_id): Extension<OwnerId>,
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
    info!("Pausing trigger {} for owner {}", id, owner_id);

    let mut scheduler = state.scheduler_for_trigger(&id).await?;
    let trigger = scheduler
        .pause_trigger(PauseTriggerRequest {
            owner_id: owner_id.0.clone(),
            id: id.0,
        })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: Trigger = trigger.into();

    Ok((StatusCode::OK, response_headers, Json(trigger)).into_response())
}
