use std::sync::Arc;

use axum::extract::{Query, State};
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ActionAttemptLog, ProjectId, RunId};
use validator::Validate;

use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::paginated::{Paginated, Pagination};
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    Query(pagination): Query<Pagination>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    ValidatedId(id): ValidatedId<RunId>,
) -> Result<Paginated<ActionAttemptLog>, ApiError> {
    pagination.validate()?;

    // Ensure that the run exists for better user experience
    let Some(_) = state
        .db
        .run_store
        .get_run(&project, &id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))? else {
            return Err(ApiError::NotFound(id.to_string()));
        };

    let attempts = state
        .db
        .attempt_store
        .get_attempts_for_run(&project, &id, pagination.into())
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok(Paginated::from(attempts.data, attempts.pagination))
}
