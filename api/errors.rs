use std::collections::HashMap;
use std::error::Error;

use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_with::skip_serializing_none;
use thiserror::Error;
use tracing::{error, warn};
use validator::{ValidationErrors, ValidationErrorsKind};

use crate::AppStateError;

#[skip_serializing_none]
#[derive(Serialize, Debug)]
struct ApiErrorBody {
    message: String,
    params: Option<HashMap<String, Vec<String>>>,
}

#[derive(Error, Debug)]
pub enum ApiError {
    // 400
    #[error("Malformed request: {0}")]
    BadRequest(String),

    // 404
    #[error("Resource requested was not found: {0}")]
    NotFound(String),

    // 401 Unauthorized response status code indicates that the client request
    // has not been completed because it lacks valid authentication
    // credentials for the requested resource.
    #[error("Authentication required to access this resource")]
    Unauthorized,

    // 403 Forbidden response status code indicates that the server
    // understands the request but refuses to authorize it.
    // ***
    // NOTE: DO NOT USE THIS IF A RESOURCE EXISTS BUT IS OWNED BY A DIFFERENT
    // PROJECT, USE NotFound INSTEAD.
    // ***
    #[error(
        "Authentication was successful but access to this resource is \
         forbidden"
    )]
    Forbidden,

    #[error("Resource conflict: {0}")]
    Conflict(String),

    // 415 Unsupported Media Type
    #[error("Expected request with `Content-Type: application/json`")]
    UnsupportedContentType,

    // 422 Unprocessable Entity/Content
    #[error("Request has failed validation")]
    UnprocessableContent {
        message: String,
        params: HashMap<String, Vec<String>>,
    },

    // 500 Internal Server Error
    #[error(
        "Internal server error, the error has been logged and will be \
         investigated."
    )]
    InternalServerError,
    // 503
    #[error(
        "Service is currently unavailable, please retry again in a few seconds"
    )]
    ServiceUnavailable,

    #[error("This functionality is not implemented")]
    NotImplemented,

    #[error(transparent)]
    BytesRejection(#[from] axum::extract::rejection::BytesRejection),
    // This is always 503 Service Unavailable!
    #[error(transparent)]
    AppStateError(#[from] AppStateError),
}

impl ApiError {
    pub fn unprocessable_content_naked(message: &str) -> Self {
        ApiError::UnprocessableContent {
            message: message.to_owned(),
            params: Default::default(),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            | ApiError::BadRequest(..) => StatusCode::BAD_REQUEST,
            | ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            | ApiError::Forbidden => StatusCode::FORBIDDEN,
            | ApiError::NotFound(..) => StatusCode::NOT_FOUND,
            | ApiError::Conflict(..) => StatusCode::CONFLICT,
            | ApiError::UnsupportedContentType => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }
            | ApiError::InternalServerError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            | ApiError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            | ApiError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            | ApiError::UnprocessableContent { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            | ApiError::BytesRejection(e) => e.status(),
            | ApiError::AppStateError(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ApiError {
    #[tracing::instrument]
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let body = match self {
            | Self::UnprocessableContent { message, params } => {
                ApiErrorBody {
                    message,
                    params: if params.is_empty() {
                        None
                    } else {
                        Some(params)
                    },
                }
            }
            | Self::BytesRejection(e) => {
                ApiErrorBody {
                    message: e.body_text(),
                    params: None,
                }
            }
            | e => {
                ApiErrorBody {
                    message: e.to_string(),
                    params: None,
                }
            }
        };
        (status_code, Json(body)).into_response()
    }
}

#[allow(clippy::wildcard_in_or_patterns)]
impl From<tonic::Status> for ApiError {
    fn from(value: tonic::Status) -> Self {
        match value.code() {
            tonic::Code::NotFound => ApiError::NotFound(value.message().to_string()),
            // Indicates a non-retryable logical error in the system.
            // An operation cannot be performed.
            tonic::Code::FailedPrecondition => {
                ApiError::unprocessable_content_naked(value.message())
            },
            tonic::Code::AlreadyExists => {
                ApiError::Conflict(value.message().to_string())
            },
            tonic::Code::Ok => {
                // We should not expect to have Status::Ok as an error!
                error!(
                grpc_code = ?value.code(),
                grpc_message = ?value.message(),
                "How did we end up here? we should not see Status::Ok wrapped \
                in an error!"
                );
                ApiError::InternalServerError
            }
            // GRPC service is not available, log this and report to the user.
            // Timeout! It's a sad day for humanity :sadface:
            tonic::Code::DeadlineExceeded
            | tonic::Code::Unavailable
            | tonic::Code::ResourceExhausted => {
                error!(
                grpc_code = ?value.code(),
                grpc_message = ?value.message(),
                "ServiceUnavailable reported due to error reported from GRPC response"
                );
                ApiError::ServiceUnavailable
            },
            // We should not expect to see those errors. If we do, we should
            // just tell the user and generate a debug key
            | tonic::Code::Internal
            // All validation should happen on API side, if we should not expect
            // an `InvalidArgument` to be triggered from user input, therefore,
            // we treat this as an internal error and we log the details for
            //
            // Change this to report BadRequest or UnprocessableContent if you want to use it to
            // report non-validation input errors.
            | tonic::Code::InvalidArgument
            | _ => {
                error!(
                grpc_code = ?value.code(),
                grpc_message = ?value.message(),
                "InternalServerError reported due to error from GRPC response"
                );
                 ApiError::InternalServerError
            },
        }
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(value: ValidationErrors) -> Self {
        let mut params = HashMap::new();
        for (key, err) in value.errors() {
            let errors = format_validation_errors(key, err);
            params.extend(errors)
        }

        ApiError::UnprocessableContent {
            message: "Request body has failed validation".to_owned(),
            params,
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(value: JsonRejection) -> Self {
        match value {
            // Request body is syntactically valid but couldn't be deserialised
            // into the target type.
            | JsonRejection::JsonDataError(e) => {
                let params = get_serde_error_params(&e);
                ApiError::UnprocessableContent {
                    message: "JSON input is valid but doesn't conform to the \
                              API shape"
                        .to_owned(),
                    params,
                }
            }
            // Json syntax error.
            | JsonRejection::JsonSyntaxError(e) => {
                ApiError::BadRequest(format!(
                    "Invalid JSON syntax, reason: {}",
                    e.source().unwrap()
                ))
            }
            // Content-Type header is missing or not `application/json`.
            | JsonRejection::MissingJsonContentType(..) => {
                ApiError::UnsupportedContentType
            }
            // Used when the request body is too large, buffering error, invalid
            // UTF-8.
            | JsonRejection::BytesRejection(e) => ApiError::BytesRejection(e),
            // JsonRejection is non-exhaustive, we must cover _.
            | _ => {
                error!("Unexpected JsonRejection: {:?}", value);
                ApiError::InternalServerError
            }
        }
    }
}

// attempt to extract the inner `serde::de::value::Error`, if that succeeds we
// can provide a more specific error
fn get_serde_error_params<'a>(
    err: &'a (dyn Error + 'static),
) -> HashMap<String, Vec<String>> {
    let mut params = HashMap::new();
    if let Some(serde_err) =
        find_error_source::<serde_path_to_error::Error<serde_json::Error>>(err)
    {
        params.insert(
            serde_err.path().to_string(),
            vec![serde_err.inner().to_string()],
        );
    }
    params
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

fn format_validation_errors(
    path: &str,
    errs: &ValidationErrorsKind,
) -> HashMap<String, Vec<String>> {
    let mut failures = HashMap::new();

    match errs {
        // Various errors on a single field, we collect.
        | ValidationErrorsKind::Field(errs) => {
            let err_col: Vec<String> =
                errs.iter().map(ToString::to_string).collect();
            failures.insert(path.into(), err_col);
        }
        // Nested errors in a struct, we flatten.
        | ValidationErrorsKind::Struct(errs) => {
            failures.extend(format_struct(errs, path));
        }
        // Errors in a list, we add the index to the path to flatten.
        | ValidationErrorsKind::List(errs) => {
            for (idx, err) in errs.iter() {
                let base_path = format!("{path}[{idx}]");
                failures.extend(format_struct(err, &base_path));
            }
        }
    };

    failures
}

fn format_struct(
    errs: &ValidationErrors,
    path: &str,
) -> HashMap<String, Vec<String>> {
    let mut failures = HashMap::new();
    for (key, err) in errs.errors() {
        let base_path = format!("{path}.{key}");
        failures.extend(format_validation_errors(&base_path, err));
    }
    failures
}
