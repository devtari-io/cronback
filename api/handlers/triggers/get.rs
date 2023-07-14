use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{
    ProjectId,
    RequestId,
    Trigger,
    TriggerId,
    TriggerManifest,
    ValidId,
};
use proto::scheduler_proto::{GetTriggerRequest, ListTriggersRequest};
use validator::Validate;

use crate::api_model::{paginate, Pagination};
use crate::errors::ApiError;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Path(id): Path<TriggerId>,
    Extension(project): Extension<ProjectId>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    if !id.is_valid() {
        return Err(ApiError::NotFound(id.to_string()));
    }
    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    let trigger = scheduler
        .get_trigger(GetTriggerRequest {
            project_id: project.0.clone(),
            id: id.0,
        })
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
    pagination: Option<Query<Pagination<TriggerId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    let triggers = scheduler
        .list_triggers(ListTriggersRequest {
            project_id: project.0.clone(),
            limit: limit as u64,
            before: pagination.before.clone().map(Into::into),
            after: pagination.after.clone().map(Into::into),
        })
        .await?
        .into_inner()
        .triggers;

    let triggers: Vec<TriggerManifest> =
        triggers.into_iter().map(Into::into).collect();

    Ok((StatusCode::OK, Json(paginate(triggers, pagination))))
}
