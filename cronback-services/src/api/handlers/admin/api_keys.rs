use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{debug_handler, Extension, Json};
use cronback_api_model::admin::{
    APIKeyMetaData,
    ApiKey,
    CreateAPIKeyResponse,
    CreateAPIkeyRequest,
};
use lib::prelude::*;
use proto::common::PaginationOut;

use crate::api::errors::ApiError;
use crate::api::extractors::ValidatedJson;
use crate::api::paginated::Paginated;
use crate::api::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn create(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    ValidatedJson(req): ValidatedJson<CreateAPIkeyRequest>,
) -> Result<Json<CreateAPIKeyResponse>, ApiError> {
    let key = state.authenticator.gen_key(req, &project).await?;

    // This is the only legitimate place where this function should be used.
    let key_str = key.unsafe_to_string();
    Ok(Json(CreateAPIKeyResponse { key: key_str }))
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
) -> Result<Paginated<ApiKey>, ApiError> {
    let keys = state
        .authenticator
        .list_keys(&project)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?
        .into_iter()
        .map(|k| {
            ApiKey {
                name: k.name,
                id: k.key_id,
                created_at: k.created_at,
                metadata: APIKeyMetaData {
                    creator_user_id: k.metadata.creator_user_id,
                },
            }
        })
        .collect::<Vec<_>>();

    Ok(Paginated::from(
        keys,
        PaginationOut {
            has_more: false,
            next_cursor: None,
        },
    ))
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn revoke(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
) -> Result<StatusCode, ApiError> {
    let deleted = state
        .authenticator
        .revoke_key(&id, &project)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(ApiError::NotFound(id));
    }

    Ok(StatusCode::OK)
}
