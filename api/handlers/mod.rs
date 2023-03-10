use axum::{
    extract::{
        rejection::{FormRejection, JsonRejection},
        FromRequest,
    },
    http::{header::CONTENT_TYPE, Request},
    Form, Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::errors::ApiError;

pub(crate) mod install_trigger;

// Parse input as form or json based on the content-type of the request.
// Note, this doesn't not perform any validation. For validated form/json, please
// use ValidatedFormOrJson
pub(crate) struct FormOrJson<T>(T);

#[axum::async_trait]
impl<S, B, T> FromRequest<S, B> for FormOrJson<T>
where
    B: Send + 'static,
    S: Send + Sync,
    Json<T>: FromRequest<S, B, Rejection = JsonRejection>,
    Form<T>: FromRequest<S, B, Rejection = FormRejection>,
    T: 'static,
{
    type Rejection = ApiError;

    async fn from_request(
        req: Request<B>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type =
            content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let Json(value) = Json::<T>::from_request(req, state).await?;
                return Ok(Self(value));
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let Form(value) = Form::<T>::from_request(req, state).await?;
                return Ok(Self(value));
            }
        }

        Err(ApiError::UnsupportedContentType(
            content_type.unwrap_or("unknown").to_string(),
        ))
    }
}

// Form Validation
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedForm<T>(pub T);

#[axum::async_trait]
impl<T, S, B> FromRequest<S, B> for ValidatedForm<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Form<T>: FromRequest<S, B, Rejection = FormRejection>,
    B: Send + 'static,
{
    type Rejection = ApiError;

    async fn from_request(
        req: Request<B>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedForm(value))
    }
}

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

pub(crate) struct ValidatedFormOrJson<T>(T);

#[axum::async_trait]
impl<T, S, B> FromRequest<S, B> for ValidatedFormOrJson<T>
where
    B: Send + 'static,
    S: Send + Sync,
    ValidatedJson<T>: FromRequest<S, B, Rejection = ApiError>,
    ValidatedForm<T>: FromRequest<S, B, Rejection = ApiError>,
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(
        req: Request<B>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type =
            content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let ValidatedJson(value) =
                    ValidatedJson::<T>::from_request(req, state).await?;
                return Ok(Self(value));
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let ValidatedForm(value) =
                    ValidatedForm::<T>::from_request(req, state).await?;
                return Ok(Self(value));
            }
        }

        Err(ApiError::UnsupportedContentType(
            content_type.unwrap_or("unknown").to_string(),
        ))
    }
}
