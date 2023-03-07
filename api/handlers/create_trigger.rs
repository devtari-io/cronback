use std::sync::Arc;

use axum::{
    debug_handler,
    extract::State,
    http::{header::HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use shared::model_util::generate_model_id;

use crate::model::trigger::Trigger;
use crate::{errors::ApiError, AppState};

use super::ValidatedJson;

#[tracing::instrument(skip_all)]
#[debug_handler]
pub(crate) async fn create_trigger(
    state: State<Arc<AppState>>,
    ValidatedJson(mut request): ValidatedJson<Trigger>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());
    // TODO: Set owner id
    request.id = generate_model_id("trig", Some("ab1"));
    // Decide the scheduler cell
    let scheduler = state.pick_scheduler("".to_string()).await?;
    // Send the request to the scheduler
    dbg!(scheduler);

    Ok((StatusCode::CREATED, response_headers, Json(Some(request))))
}
