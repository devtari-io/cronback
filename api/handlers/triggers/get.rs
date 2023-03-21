use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Json};
use proto::scheduler_proto::GetTriggerRequest;
use serde_json::json;
use shared::types::{OwnerId, Trigger, TriggerId, ValidId};
use tracing::info;

use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Path(id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    // TODO: Get owner id
    let owner_id = OwnerId::from("ab1".to_owned());
    if !id.is_valid() {
        return Ok((
            StatusCode::BAD_REQUEST,
            response_headers,
            // TODO: We need a proper API design for API errors
            Json("Invalid trigger id"),
        )
            .into_response());
    }
    info!("Get trigger {} for owner {}", id, owner_id);

    let mut scheduler = state.scheduler_for_trigger(&id).await?;
    let trigger = scheduler
        .get_trigger(GetTriggerRequest { id: id.0 })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: Trigger = trigger.into();
    Ok((StatusCode::OK, response_headers, Json(trigger)).into_response())
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    state: State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    // TODO: Get owner id
    let owner_id = OwnerId::from("ab1".to_owned());
    info!("Get all trigger for owner {}", owner_id);

    let triggers = state
        .db
        .trigger_store
        .get_triggers_by_owner(&owner_id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        response_headers,
        Json(json!({ "triggers": triggers })),
    ))
}
