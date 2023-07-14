use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{AttemptLogId, InvocationId, ProjectId, ValidId};
use validator::Validate;

use crate::api_model::{paginate, Pagination};
use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<AttemptLogId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
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

    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

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

    if invocation.project != project {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let attempts = state
        .db
        .attempt_store
        .get_attempts_for_invocation(
            &id,
            pagination.before.clone(),
            pagination.after.clone(),
            limit,
        )
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        response_headers,
        Json(paginate(attempts, pagination)),
    )
        .into_response())
}
