use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use serde_json::json;
use shared::types::{OwnerId, TriggerId};
use tracing::info;

use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    state: State<Arc<AppState>>,
    Extension(owner_id): Extension<OwnerId>,
    Path(trigger_id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    info!("Get all invocations for owner {}", owner_id);

    let Some(trigger) = state
        .db
        .trigger_store
        .get_trigger(&trigger_id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))? else {
            return Ok(
                StatusCode::NOT_FOUND.into_response()
            );
        };

    if trigger.owner_id != owner_id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let invocations = state
        .db
        .invocation_store
        .get_invocations_by_trigger(&trigger_id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        response_headers,
        Json(json!({ "invocations": invocations })),
    )
        .into_response())
}
