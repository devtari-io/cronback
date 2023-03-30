use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use shared::types::{InvocationId, OwnerId, TriggerId};
use tracing::info;
use validator::Validate;

use crate::api_model::{paginate, Pagination};
use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<InvocationId>>>,
    state: State<Arc<AppState>>,
    Extension(owner_id): Extension<OwnerId>,
    Path(trigger_id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    info!("Get all invocations for owner {}", owner_id);
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

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

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let invocations = state
        .db
        .invocation_store
        .get_invocations_by_trigger(
            &trigger_id,
            pagination.before.clone(),
            pagination.after.clone(),
            limit,
        )
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        response_headers,
        Json(paginate(invocations, pagination)),
    )
        .into_response())
}
