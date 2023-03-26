use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use serde_json::json;
use shared::types::{InvocationId, OwnerId, ValidId};
use tracing::info;

use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    state: State<Arc<AppState>>,
    Extension(owner_id): Extension<OwnerId>,
    Path(id): Path<InvocationId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());

    if !id.is_valid() {
        return Ok((
            StatusCode::BAD_REQUEST,
            response_headers,
            // TODO: We need a proper API design for API errors
            Json("Invalid invocation id"),
        )
            .into_response());
    }

    info!("Get all attempts for invocation {}", id);

    let Some(invocation) = state
        .db
        .invocation_store
        .get_invocation(&id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))? else {
            return Ok(
                StatusCode::NOT_FOUND.into_response()
            );
        };

    if invocation.owner_id != owner_id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let attempts = state
        .db
        .attempt_store
        .get_attempts_for_invocation(&id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        response_headers,
        Json(json!({ "attempts": attempts })),
    )
        .into_response())
}
