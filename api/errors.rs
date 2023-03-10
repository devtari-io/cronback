use std::error::Error;

use axum::extract::rejection::JsonRejection;
use axum::{
    extract::rejection::FormRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use thiserror::Error;
use tonic::Status;

use crate::AppStateError;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(
        "Unsupported content-type '{0}'. Only application/json or \
            x-www-form-urlencoded can be used here"
    )]
    UnsupportedContentType(String),
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),
    #[error(transparent)]
    FormRejection(#[from] FormRejection),
    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),
    #[error(transparent)]
    AppStateError(#[from] AppStateError),
    #[error(transparent)]
    SchedulerError(#[from] Status),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            | ApiError::ValidationError(_) => {
                let message = format!("Input validation error: [{}]", self)
                    .replace('\n', ", ");
                (StatusCode::BAD_REQUEST, message)
            }
            // Form Rejections, we are expanding the match to expose the underlying
            // error better to our users. See https://docs.rs/axum/latest/axum/extract/index.html#accessing-inner-errors for details.
            | ApiError::FormRejection(
                FormRejection::FailedToDeserializeFormBody(err),
            ) => serde_error_response(err),
            | ApiError::FormRejection(
                FormRejection::FailedToDeserializeForm(err),
            ) => serde_error_response(err),
            | ApiError::FormRejection(err) => {
                (StatusCode::BAD_REQUEST, format!("Form rejected: {err:?}"))
            }
            | ApiError::JsonRejection(
                JsonRejection::MissingJsonContentType(e),
            ) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string()),
            | ApiError::JsonRejection(e) => serde_json_error_response(e),
            | ApiError::UnsupportedContentType(_) => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, self.to_string())
            }
            | ApiError::AppStateError(e) => {
                (StatusCode::SERVICE_UNAVAILABLE, e.to_string())
            }
            | ApiError::SchedulerError(e) => {
                // TODO: Ship GRPC errors better
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        }
        .into_response()
    }
}

// attempt to extract the inner `serde_json::Error`, if that succeeds we can
// provide a more specific error
fn serde_json_error_response<E>(err: E) -> (StatusCode, String)
where
    E: Error + 'static,
{
    if let Some(serde_json_err) = find_error_source::<serde_json::Error>(&err) {
        (
            StatusCode::BAD_REQUEST,
            format!("JSON validation error: {}", serde_json_err.to_string(),),
        )
    } else {
        (StatusCode::BAD_REQUEST, "Unknown error".to_string())
    }
}

// attempt to extract the inner `serde_json::Error`, if that succeeds we can
// provide a more specific error
fn serde_error_response<E>(err: E) -> (StatusCode, String)
where
    E: Error + 'static,
{
    if let Some(serde_err) = find_error_source::<serde::de::value::Error>(&err)
    {
        (
            StatusCode::BAD_REQUEST,
            format!("Form validation error: {}", serde_err.to_string(),),
        )
    } else {
        (StatusCode::BAD_REQUEST, "Unknown error".to_string())
    }
}
// attempt to downcast `err` into a `T` and if that fails recursively try and
// downcast `err`'s source
fn find_error_source<'a, T>(err: &'a (dyn Error + 'static)) -> Option<&'a T>
where
    T: Error + 'static,
{
    if let Some(err) = err.downcast_ref::<T>() {
        Some(err)
    } else if let Some(source) = err.source() {
        find_error_source(source)
    } else {
        None
    }
}
