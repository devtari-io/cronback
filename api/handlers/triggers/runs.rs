use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::ProjectId;
use validator::Validate;

use crate::errors::ApiError;
use crate::model::Run;
use crate::paginated::{Paginated, Pagination};
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    Query(pagination): Query<Pagination>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Path(name): Path<String>,
) -> Result<Paginated<Run>, ApiError> {
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

    let runs = state
        .db
        .run_store
        .get_runs_by_trigger(&project, &trigger_id, pagination.into())
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    let paginated_out = runs.pagination;

    // Fake conversion to proto then back to API model until this moves to
    // dispatcher/scheduler.
    let runs: Vec<proto::run_proto::Run> =
        runs.data.into_iter().map(Into::into).collect();

    let runs: Vec<Run> = runs.into_iter().map(Into::into).collect();

    let runs = Paginated::from(runs, paginated_out);
    Ok(runs)
}
