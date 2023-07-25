use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId};

use crate::api::errors::ApiError;
use crate::api::extractors::ValidatedJson;
use crate::api::model::UpsertTriggerRequest;
use crate::api::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn put(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(request): ValidatedJson<UpsertTriggerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // An important detail to note. The name in the URL is the _original_ name,
    // if the name in the request body is different, we will update the stored
    // name to the new one unless it already exists.
    // Validate that the internal name is the same as in url
    super::install::install_or_update(
        state,
        request_id,
        // Intention here is to update an existing, or create a new one.
        /* request_precondition = */
        None,
        project,
        Some(name),
        request,
    )
    .await
}
