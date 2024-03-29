use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::prelude::*;
use proto::scheduler_svc::RunTriggerRequest;

use crate::api::api_model::{Run, RunTrigger};
use crate::api::errors::ApiError;
use crate::api::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn run(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    Path(name): Path<String>,
    // The body of the request is optional, so we use Option<Json<...>>.
    request: Option<Json<RunTrigger>>,
) -> Result<impl IntoResponse, ApiError> {
    dbg!(&request);
    let Json(request) = request.unwrap_or_default();
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let run_request = RunTriggerRequest {
        name,
        mode: request.mode.into(),
    };
    let run = scheduler
        .run_trigger(run_request)
        .await?
        .into_inner()
        .run
        .unwrap();
    let run: Run = run.into();

    Ok((StatusCode::CREATED, Json(run)).into_response())
}
