use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId};
use proto::scheduler_proto::PauseTriggerRequest;

use crate::api_srv::errors::ApiError;
use crate::api_srv::model::Trigger;
use crate::api_srv::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn pause(
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
        .pause_trigger(PauseTriggerRequest { name })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: Trigger = trigger.into();

    Ok((StatusCode::OK, Json(trigger)).into_response())
}
