use async_trait::async_trait;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{FromRequest, FromRequestParts, Path};
use axum::http::request::Parts;
use axum::http::Request;
use axum::Json;
use lib::model::{ModelId, ValidShardedId};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::errors::ApiError;

// Json Input Validation
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T, S, B> FromRequest<S, B> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S, B, Rejection = JsonRejection>,
    B: Send + 'static,
{
    type Rejection = ApiError;

    async fn from_request(
        req: Request<B>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

#[derive(Debug)]
pub struct ValidatedId<T>(pub ValidShardedId<T>);

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedId<T>
where
    T: DeserializeOwned + Send + ModelId,
    S: Send + Sync,
    Path<T>: FromRequestParts<S, Rejection = PathRejection>,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let raw_id = value.value().to_owned();
        let validated = value.validated().map_err(|_|
            // We know the id is invalid so we won't bother even querying the
            // database for it.
            ApiError::NotFound(raw_id))?;

        Ok(ValidatedId(validated))
    }
}
