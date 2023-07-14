use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId};

use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::model::UpsertTriggerRequest;
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn put(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(mut request): ValidatedJson<UpsertTriggerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Intention here is to update an existing, or create a new one.
    let fail_if_exists = false;
    request.trigger.name = Some(name.clone());
    // Validate that the internal name is the same as in url
    if request.trigger.name.is_some()
        && &name != request.trigger.name.as_ref().unwrap()
    {
        return Err(ApiError::unprocessable_content_naked(
            "Trigger name in body doesn't match the one in url",
        ));
    }
    crate::handlers::triggers::install::install_or_update(
        state,
        request_id,
        fail_if_exists,
        project,
        request,
    )
    .await
}
