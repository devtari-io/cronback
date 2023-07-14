use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId, TriggerId, TriggerManifest};
use proto::scheduler_proto::ResumeTriggerRequest;

use crate::errors::ApiError;
use crate::extractors::ValidatedId;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn resume(
    state: State<Arc<AppState>>,
    ValidatedId(id): ValidatedId<TriggerId>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let mut scheduler = state.get_scheduler(&request_id, &project).await?;
    let trigger = scheduler
        .resume_trigger(ResumeTriggerRequest {
            project_id: project.into(),
            id: id.into(),
        })
        .await?
        .into_inner()
        .trigger
        .unwrap();

    let trigger: TriggerManifest = trigger.into();

    Ok((StatusCode::OK, Json(trigger)).into_response())
}
