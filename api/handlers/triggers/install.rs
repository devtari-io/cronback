use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId, Schedule, Trigger, TriggerId};
use proto::scheduler_proto::InstallTriggerRequest;
use tracing::error;

use crate::api_model::InstallTrigger;
use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn install(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(request): ValidatedJson<InstallTrigger>,
) -> Result<impl IntoResponse, ApiError> {
    // TODO (AhmedSoliman): Make this configurable via a HEADER
    let fail_if_exists = false;
    install_or_update(state, None, request_id, fail_if_exists, project, request)
        .await
}

pub(crate) async fn install_or_update(
    state: Arc<AppState>,
    id: Option<ValidShardedId<TriggerId>>,
    request_id: RequestId,
    fail_if_exists: bool,
    project: ValidShardedId<ProjectId>,
    mut request: InstallTrigger,
) -> Result<impl IntoResponse, ApiError> {
    // If we have an Id already, we must allow updates.
    if id.is_some() && fail_if_exists {
        error!(
            trigger_id = ?id,
            "Bad request: fail_if_exists is true, but we have an Id already."
        );

        return Err(ApiError::ServiceUnavailable);
    }
    // Decide the scheduler cell
    // Pick cell.
    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    // patch install trigger until we have a better way to do this.
    if let Some(Schedule::Recurring(cron)) = request.schedule.as_mut() {
        if cron.limit > 0 {
            cron.remaining = cron.limit;
        }
    };
    // The spinner will update `remaining` to the accurate value as soon as it
    // runs.
    if let Some(Schedule::RunAt(run_at)) = request.schedule.as_mut() {
        run_at.remaining = run_at.timepoints.len() as u64;
    };
    // Send the request to the scheduler
    let install_request: InstallTriggerRequest =
        request.into_proto(project, id, fail_if_exists);

    let response = scheduler
        .install_trigger(install_request)
        .await?
        .into_inner();
    let trigger: Trigger = response.trigger.unwrap().into();
    let status = if response.already_existed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    Ok((status, Json(trigger)))
}
