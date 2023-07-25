use std::sync::Arc;

use axum::extract::{Path, State};
use axum::{Extension, Json};
use lib::prelude::*;
use proto::scheduler_svc::CancelTriggerRequest;

use crate::api::api_model::Trigger;
use crate::api::errors::ApiError;
use crate::api::AppState;

#[tracing::instrument(skip(state))]
pub(crate) async fn cancel(
    state: State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Trigger>, ApiError> {
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let trigger = scheduler
        .cancel_trigger(CancelTriggerRequest { name })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: Trigger = trigger.into();

    Ok(Json(trigger))
}
