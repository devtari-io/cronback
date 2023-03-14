use std::sync::Arc;

use axum::{
    debug_handler,
    extract::State,
    http::{header::HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use proto::scheduler_proto::InstallTriggerRequest;

use crate::api_model::InstallTrigger;
use crate::extractors::ValidatedJson;
use crate::{errors::ApiError, AppState};
use shared::types::{OwnerId, Trigger};

#[tracing::instrument(skip_all)]
#[debug_handler]
pub(crate) async fn install(
    state: State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<InstallTrigger>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    // TODO: Get owner id
    let owner_id = OwnerId::from("ab1".to_owned());
    // Decide the scheduler cell
    // Pick cell.
    let (cell_id, mut scheduler) = state.pick_scheduler("".to_string()).await?;
    // Send the request to the scheduler
    let trigger = scheduler
        .install_trigger(InstallTriggerRequest {
            install_trigger: Some(request.into_proto(owner_id, cell_id)),
        })
        .await?
        .into_inner()
        .trigger
        .unwrap();
    let trigger: Trigger = trigger.into();

    Ok((StatusCode::CREATED, response_headers, Json(trigger)))
}
