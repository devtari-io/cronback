use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RunId};
use proto::common::PaginationIn;
use validator::Validate;

use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::model::Run;
use crate::paginated::{Paginated, Pagination};
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    ValidatedId(id): ValidatedId<RunId>,
) -> Result<impl IntoResponse, ApiError> {
    let run = state
        .db
        .run_store
        .get_run(&project, &id)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    let Some(run) = run else {
            return Err(ApiError::NotFound(id.to_string()));
    };

    // Fake conversion to proto then back to API model until this moves to
    // dispatcher/scheduler.
    let run: proto::run_proto::Run = run.into();
    let run: Run = run.into();

    Ok((StatusCode::OK, Json(run)))
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    Query(pagination): Query<Pagination>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
) -> Result<Paginated<Run>, ApiError> {
    pagination.validate()?;

    // Fake conversion to proto.
    let pagination: PaginationIn = pagination.into();

    let runs = state
        .db
        .run_store
        .get_runs_by_project(&project, pagination)
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
