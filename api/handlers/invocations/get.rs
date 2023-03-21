use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Json};
use serde_json::json;
use shared::types::{InvocationId, OwnerId, ValidId};
use tracing::info;

use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Path(id): Path<InvocationId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    info!("Get invocation with id {}", id);

    // TODO: authorization

    if !id.is_valid() {
        return Ok((
            StatusCode::BAD_REQUEST,
            response_headers,
            // TODO: We need a proper API design for API errors
            Json("Invalid invocation id"),
        )
            .into_response());
    }

    let invocation = state
        .db
        .invocation_store
        .get_invocation(&id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok(match invocation {
        | Some(invocation) => {
            (StatusCode::OK, response_headers, Json(invocation)).into_response()
        }
        | None => {
            (
                StatusCode::NOT_FOUND,
                response_headers,
                // TODO: We need a proper API design for API errors
                Json("Invocation not found"),
            )
                .into_response()
        }
    })
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
    info!("Get all invocations for owner {}", owner_id);

    let invocations = state
        .db
        .invocation_store
        .get_invocations_by_owner(&owner_id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        response_headers,
        Json(json!({ "invocations": invocations })),
    ))
}
