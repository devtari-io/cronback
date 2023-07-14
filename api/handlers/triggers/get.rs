use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use axum_extra::extract::Query;
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId};
use proto::scheduler_proto::{GetTriggerRequest, ListTriggersRequest};
use validator::Validate;

use crate::errors::ApiError;
use crate::model::{ListFilters, Trigger};
use crate::paginated::{Paginated, Pagination};
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let trigger = scheduler
        .get_trigger(GetTriggerRequest { name })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: Trigger = trigger.into();
    Ok((StatusCode::OK, Json(trigger)).into_response())
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    Query(pagination): Query<Pagination>,
    Query(filters): Query<ListFilters>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Paginated<Trigger>, ApiError> {
    pagination.validate()?;

    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let response = scheduler
        .list_triggers(ListTriggersRequest {
            pagination: Some(pagination.into()),
            filter: Some(filters.into()),
        })
        .await?
        .into_inner();

    Ok(Paginated::from(
        response.triggers,
        response.pagination.unwrap_or_default(),
    ))
}
