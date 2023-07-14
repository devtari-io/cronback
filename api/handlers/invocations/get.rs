use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{InvocationId, ProjectId};
use validator::Validate;

use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::model::{paginate, Pagination};
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    ValidatedId(id): ValidatedId<InvocationId>,
) -> Result<impl IntoResponse, ApiError> {
    let invocation = state
        .db
        .invocation_store
        .get_invocation(&project, &id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    let Some(invocation) = invocation else {
            return Err(ApiError::NotFound(id.to_string()));
    };

    Ok((StatusCode::OK, Json(invocation)).into_response())
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<InvocationId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let invocations = state
        .db
        .invocation_store
        .get_invocations_by_project(
            &project,
            pagination.before.clone(),
            pagination.after.clone(),
            limit,
        )
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(paginate(invocations, pagination))))
}
