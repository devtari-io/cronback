use std::sync::Arc;

use axum::extract::State;
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId, TriggerId};
use tracing::error;

use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::model::{InstallTriggerRequest, InstallTriggerResponse};
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn install(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(request): ValidatedJson<InstallTriggerRequest>,
) -> Result<InstallTriggerResponse, ApiError> {
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
    request: InstallTriggerRequest,
) -> Result<InstallTriggerResponse, ApiError> {
    // If we have an Id already, we must allow updates.
    if id.is_some() && fail_if_exists {
        error!(
            trigger_id = ?id,
            "Bad request: fail_if_exists is true, but we have an Id already."
        );

        return Err(ApiError::ServiceUnavailable);
    }
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let install_request = request.into_proto(id, fail_if_exists);

    let response = scheduler
        .install_trigger(install_request)
        .await?
        .into_inner();

    let response = InstallTriggerResponse {
        trigger: response.trigger.unwrap().into(),
        already_existed: response.already_existed,
    };
    Ok(response)
}
