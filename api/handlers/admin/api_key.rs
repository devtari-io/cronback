use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{debug_handler, Json};
use lib::types::OwnerId;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::ApiKey;
use crate::errors::ApiError;
use crate::extractors::ValidatedJson;
use crate::{AppState, AppStateError};

#[derive(Deserialize, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateAPIkeyRequest {
    owner_id: String,

    #[validate(length(
        min = 2,
        max = 30,
        message = "name must be between 2 and 30 characters"
    ))]
    key_name: String,
}

#[tracing::instrument(skip(state))]
#[debug_handler]
pub(crate) async fn create(
    state: State<Arc<AppState>>,
    ValidatedJson(req): ValidatedJson<CreateAPIkeyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let key = ApiKey::generate();

    // This is the only legitimate place where this function should be used.
    let key_str = key.unsafe_to_string();

    state
        .db
        .auth_store
        .save_key(key, &OwnerId::from(req.owner_id), &req.key_name)
        .await
        .map_err(|e| AppStateError::DatabaseError(e.to_string()))?;

    #[derive(Serialize)]
    struct Response {
        key: String,
    }

    Ok(Json(Response { key: key_str }))
}
