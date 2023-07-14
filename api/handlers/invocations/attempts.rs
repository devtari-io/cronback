use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{AttemptLogId, InvocationId, ProjectId};
use validator::Validate;

use crate::api_model::{paginate, Pagination};
use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<AttemptLogId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
    ValidatedId(id): ValidatedId<InvocationId>,
) -> Result<impl IntoResponse, ApiError> {
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

    // TODO: Remove after changing all operations to query by project id.
    if invocation.project != project {
        return Err(ApiError::NotFound(id.to_string()));
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

    Ok((StatusCode::OK, Json(paginate(attempts, pagination))).into_response())
}
