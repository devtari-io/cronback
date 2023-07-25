use std::sync::Arc;

use axum::extract::State;
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId};
use proto::common::request_precondition::PreconditionType;
use proto::common::{RequestPrecondition, UpsertEffect};
use tracing::error;

use crate::api::errors::ApiError;
use crate::api::extractors::ValidatedJson;
use crate::api::model::{UpsertTriggerRequest, UpsertTriggerResponse};
use crate::api::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn install(
    State(state): State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(mut request): ValidatedJson<UpsertTriggerRequest>,
) -> Result<UpsertTriggerResponse, ApiError> {
    // Intention here is to install a new one, no updates to existing triggers
    // should be allowed.
    let request_precondition = RequestPrecondition {
        precondition_type: PreconditionType::MustNotExist.into(),
        etag: None,
    };

    // We generate a name for you if you didn't specify one already.
    request.trigger.name = request.trigger.name.or_else(|| {
        names::Generator::with_naming(names::Name::Numbered).next()
    });

    if request.trigger.name.is_none() {
        // We couldn't generate a name for some reason!
        error!(
            request = ?request,
            "Failed to generate a name for the trigger."
        );
        return Err(ApiError::unprocessable_content_naked(
            "Trigger name cannot be set through the request body.",
        ));
    }
    install_or_update(
        state,
        request_id,
        Some(request_precondition),
        project,
        /* existing_name = */ None,
        request,
    )
    .await
}

pub(crate) async fn install_or_update(
    state: Arc<AppState>,
    request_id: RequestId,
    precondition: Option<RequestPrecondition>,
    project: ValidShardedId<ProjectId>,
    existing_name: Option<String>,
    request: UpsertTriggerRequest,
) -> Result<UpsertTriggerResponse, ApiError> {
    // If we have an Id already, we must allow updates.
    let mut scheduler = state
        .scheduler_clients
        .get_client(&request_id, &project)
        .await?;
    let install_request = request.into_proto(existing_name, precondition);

    let response = scheduler
        .upsert_trigger(install_request)
        .await?
        .into_inner();

    let response = UpsertTriggerResponse {
        trigger: response.trigger.unwrap().into(),
        effect: UpsertEffect::from_i32(response.effect).unwrap(),
    };
    Ok(response)
}
