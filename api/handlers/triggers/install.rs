use std::sync::Arc;

use axum::extract::State;
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Json};
use proto::scheduler_proto::InstallTriggerRequest;
use shared::types::{OwnerId, Schedule, Trigger};

use crate::api_model::InstallTrigger;
use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::AppState;

#[tracing::instrument(skip_all)]
#[debug_handler]
pub(crate) async fn install(
    state: State<Arc<AppState>>,
    ValidatedJson(mut request): ValidatedJson<InstallTrigger>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());

    // TODO: Get owner id
    let owner_id = OwnerId::from("ab1".to_owned());
    // Decide the scheduler cell
    // Pick cell.
    let (cell_id, mut scheduler) = state.pick_scheduler("".to_string()).await?;
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
        request.into_proto(owner_id, cell_id);
    let trigger = scheduler
        .install_trigger(install_request)
        .await?
        .into_inner()
        .trigger
        .unwrap();
    let trigger: Trigger = trigger.into();

    Ok((StatusCode::CREATED, response_headers, Json(trigger)))
}
