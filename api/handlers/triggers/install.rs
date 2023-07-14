use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::types::{ProjectId, RequestId, Schedule, Trigger};
use proto::scheduler_proto::InstallTriggerRequest;

use crate::api_model::InstallTrigger;
use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn install(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ProjectId>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(mut request): ValidatedJson<InstallTrigger>,
) -> Result<impl IntoResponse, ApiError> {
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
    let install_request: InstallTriggerRequest = request.into_proto(project);
    let trigger = scheduler
        .install_trigger(install_request)
        .await?
        .into_inner()
        .trigger
        .unwrap();
    let trigger: Trigger = trigger.into();

    Ok((StatusCode::CREATED, Json(trigger)))
}
