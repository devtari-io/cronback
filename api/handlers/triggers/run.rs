use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId, TriggerId};
use proto::scheduler_proto::RunTriggerRequest;

use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::model::{Run, RunTrigger};
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn run(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedId(id): ValidatedId<TriggerId>,
    // The body of the request is optional, so we use Option<Json<...>>.
    request: Option<Json<RunTrigger>>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(request) = request.unwrap_or_default();
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let run_request = RunTriggerRequest {
        project_id: project.into(),
        id: id.into(),
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
