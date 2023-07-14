use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{debug_handler, Extension};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, RequestId, TriggerId};

use crate::api_model::InstallTrigger;
use crate::errors::ApiError;
use crate::extractors::{ValidatedId, ValidatedJson};
use crate::AppState;

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn put(
    State(state): State<Arc<AppState>>,
    ValidatedId(id): ValidatedId<TriggerId>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    Extension(request_id): Extension<RequestId>,
    ValidatedJson(request): ValidatedJson<InstallTrigger>,
) -> Result<impl IntoResponse, ApiError> {
    let fail_if_exists = false;
    crate::handlers::triggers::install::install_or_update(
        state,
        Some(id),
        request_id,
        fail_if_exists,
        project,
        request,
    )
    .await
}
