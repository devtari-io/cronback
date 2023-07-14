use std::sync::Arc;

use axum::extract::State;
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId};

use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::model::{UpsertTriggerRequest, UpsertTriggerResponse};
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn install(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(mut request): ValidatedJson<UpsertTriggerRequest>,
) -> Result<UpsertTriggerResponse, ApiError> {
    // TODO (AhmedSoliman): Make this configurable via a HEADER
    // Intention here is to install a new one.
    let fail_if_exists = true;
    // Fail is the name was set through this request.
    // if request manifest is some and has a .name with some , return error
    if request.trigger.name.is_some() {
        return Err(ApiError::unprocessable_content_naked(
            "Trigger name cannot be set through the request body.",
        ));
    }

    // We generate a name for you.
    request.trigger.name = Some(
        names::Generator::with_naming(names::Name::Numbered)
            .next()
            .unwrap(),
    );
    install_or_update(state, request_id, fail_if_exists, project, request).await
}

pub(crate) async fn install_or_update(
    state: Arc<AppState>,
    request_id: RequestId,
    fail_if_exists: bool,
    project: ValidShardedId<ProjectId>,
    request: UpsertTriggerRequest,
) -> Result<UpsertTriggerResponse, ApiError> {
    // If we have an Id already, we must allow updates.
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let install_request = request.into_proto(fail_if_exists);

    let response = scheduler
        .upsert_trigger(install_request)
        .await?
        .into_inner();

    let response = UpsertTriggerResponse {
        trigger: response.trigger.unwrap().into(),
        already_existed: response.already_existed,
    };
    Ok(response)
}
