use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{ProjectId, Trigger, TriggerId, TriggerManifest, ValidId};
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
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    if !id.is_valid() {
        return Ok((
            StatusCode::BAD_REQUEST,
            response_headers,
            // TODO: We need a proper API design for API errors
            Json("Invalid trigger id"),
        )
            .into_response());
    }
    let mut scheduler = state.get_scheduler(&project).await?;
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

    if trigger.project != project {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    Ok((StatusCode::OK, response_headers, Json(trigger)).into_response())
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<TriggerId>>>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let mut scheduler = state.get_scheduler(&project).await?;
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

    Ok((
        StatusCode::OK,
        response_headers,
        Json(paginate(triggers, pagination)),
    ))
}
