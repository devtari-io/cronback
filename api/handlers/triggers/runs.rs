use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RunId};
use validator::Validate;

use crate::errors::ApiError;
use crate::model::{paginate, Pagination};
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<RunId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

    // TODO: Move to dispatcher _or_ scheduler
    // Ensure that the trigger exists for better user experience
    let Some(trigger_id) = state
        .db
        .trigger_store
        .find_trigger_id_for_name(&project, &name)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))? else {
            return Err(ApiError::NotFound(name.to_string()));
        };

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let runs = state
        .db
        .run_store
        .get_runs_by_trigger(
            &project,
            &trigger_id,
            pagination.before.clone(),
            pagination.after.clone(),
            limit,
        )
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::OK, Json(paginate(runs, pagination))).into_response())
}
