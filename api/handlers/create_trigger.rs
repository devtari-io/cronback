use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

use crate::{errors::ApiError, model::Trigger, AppState};

use super::ValidatedJson;

pub(crate) async fn create_trigger(
    _state: State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<Trigger>,
) -> Result<impl IntoResponse, ApiError> {
    let mut response_headers = HeaderMap::new();
    response_headers
        .insert("cronback-trace-id", "SOMETHING SOMETHING".parse().unwrap());

    Ok((StatusCode::CREATED, response_headers, Json(Some(request))))
}
