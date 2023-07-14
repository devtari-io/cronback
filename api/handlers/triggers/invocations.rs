use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{InvocationId, ProjectId, TriggerId};
use validator::Validate;

use crate::api_model::{paginate, Pagination};
use crate::errors::ApiError;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<InvocationId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Path(trigger_id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

    let Some(trigger) = state
        .db
        .trigger_store
        .get_trigger(&project, &trigger_id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))? else {
            return Err(ApiError::NotFound(trigger_id.to_string()));
        };

    if trigger.project != project {
        return Err(ApiError::NotFound(trigger_id.to_string()));
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

    Ok((StatusCode::OK, Json(paginate(invocations, pagination)))
        .into_response())
}
