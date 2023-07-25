use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use axum_extra::extract::Query;
use lib::prelude::*;
use proto::scheduler_svc::{GetTriggerRequest, ListTriggersRequest};
use validator::Validate;

use crate::api::api_model::{Trigger, TriggersFilter};
use crate::api::errors::ApiError;
use crate::api::paginated::{Paginated, Pagination};
use crate::api::AppState;

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
    Query(filters): Query<TriggersFilter>,
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
