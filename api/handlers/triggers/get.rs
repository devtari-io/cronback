use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use axum_extra::extract::Query;
use lib::model::ValidShardedId;
use lib::types::{
    trigger,
    ProjectId,
    RequestId,
    Trigger,
    TriggerId,
    TriggerManifest,
};
use proto::scheduler_proto::{
    GetTriggerRequest,
    ListTriggersFilter,
    ListTriggersRequest,
};
use serde::Deserialize;
use validator::Validate;

use crate::api_model::{paginate, Pagination};
use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    ValidatedId(id): ValidatedId<TriggerId>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    let trigger = scheduler
        .get_trigger(GetTriggerRequest {
            project_id: project.into(),
            id: id.into(),
        })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: Trigger = trigger.into();
    Ok((StatusCode::OK, Json(trigger)).into_response())
}

#[derive(Debug, Deserialize, Default, Validate)]
pub(crate) struct ListFilters {
    reference: Option<String>,
    #[serde(default)]
    status: Vec<trigger::Status>,
}

impl From<ListFilters> for ListTriggersFilter {
    fn from(value: ListFilters) -> Self {
        ListTriggersFilter {
            reference: value.reference,
            statuses: value.status.into_iter().map(Into::into).collect(),
        }
    }
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    pagination: Option<Query<Pagination<TriggerId>>>,
    Query(filters): Query<ListFilters>,
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;

    // Trick. We want to know if there is a next page, so we ask for one more
    let limit = pagination.limit() + 1;

    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    let triggers = scheduler
        .list_triggers(ListTriggersRequest {
            project_id: project.into(),
            limit: limit as u64,
            before: pagination.before.clone().map(Into::into),
            after: pagination.after.clone().map(Into::into),
            filter: Some(filters.into()),
        })
        .await?
        .into_inner()
        .triggers;

    let triggers: Vec<TriggerManifest> =
        triggers.into_iter().map(Into::into).collect();

    Ok((StatusCode::OK, Json(paginate(triggers, pagination))))
}
