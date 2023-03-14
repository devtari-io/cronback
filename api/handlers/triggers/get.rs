use std::sync::Arc;

use axum::{
    debug_handler,
    extract::{Path, State},
    http::{header::HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use proto::scheduler_proto::GetTriggerRequest;
use tracing::info;

use crate::{errors::ApiError, AppState};
use shared::types::{CellId, OwnerId, Trigger, TriggerId, ValidId};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn get(
    state: State<Arc<AppState>>,
    Path(id): Path<TriggerId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    // TODO: Get owner id
    let owner_id = OwnerId::from("ab1".to_owned());
    if !id.is_valid() {
        return Ok((
            StatusCode::BAD_REQUEST,
            response_headers,
            // TODO: We need a proper API design for API errors
            Json(Err("Invalid trigger id")),
        ));
    }
    info!("Get trigger {} for owner {}", id, owner_id);
    // Decide the scheduler cell
    // TODO: Now, how do we figure which scheduler has this trigger?
    // For now, we'll assume all triggers are on Cell 0
    let cell_id = CellId::from(0);
    let mut scheduler = state.scheduler(cell_id).await?;

    let trigger = scheduler
        .get_trigger(GetTriggerRequest { id: id.0 })
        .await?
        .into_inner()
        .trigger
        .unwrap();
    let trigger: Trigger = trigger.into();

    Ok((StatusCode::OK, response_headers, Json(Ok(trigger))))
}
