use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{debug_handler, Extension, Json};
use lib::model::ValidShardedId;
use lib::types::ProjectId;
use proto::common::PaginationOut;

use crate::auth::ApiKey;
use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::model::{CreateAPIKeyResponse, CreateAPIkeyRequest, ListKeysItem};
use crate::paginated::Paginated;
use crate::{AppState, AppStateError};

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn create(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
    ValidatedJson(req): ValidatedJson<CreateAPIkeyRequest>,
) -> Result<Json<CreateAPIKeyResponse>, ApiError> {
    let key = ApiKey::generate();

    // This is the only legitimate place where this function should be used.
    let key_str = key.unsafe_to_string();

    state
        .db
        .auth_store
        .save_key(key, &project, &req.key_name)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    Ok(Json(CreateAPIKeyResponse { key: key_str }))
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn list(
    state: State<Arc<AppState>>,
    Extension(project): Extension<ValidShardedId<ProjectId>>,
) -> Result<Paginated<ListKeysItem>, ApiError> {
    let keys = state
        .db
        .auth_store
        .list_keys(&project)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?
        .into_iter()
        .map(|k| {
            ListKeysItem {
                name: k.name,
                id: k.key_id,
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
        .db
        .auth_store
        .delete_key(id.clone(), &project)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(ApiError::NotFound(id));
    }

    Ok(StatusCode::OK)
}
