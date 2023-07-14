use std::collections::BTreeMap;

use http::StatusCode;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use tracing::log::warn;
use url::Url;

pub const REQUEST_ID_HEADER: &str = "x-cronback-request-id";
pub const PROJECT_ID_HEADER: &str = "x-cronback-project-id";

#[derive(Deserialize, Debug)]
struct ApiErrorBody {
    message: String,
    params: Option<BTreeMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    status_code: StatusCode,
    message: String,
    params: Option<BTreeMap<String, Vec<String>>>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "({}) {}", self.status_code, self.message)?;
        if let Some(ref params) = self.params {
            for (key, errors) in params {
                writeln!(f, "  [{}]:", key)?;
                for error in errors {
                    writeln!(f, "    - {}", error)?;
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Clone)]
pub struct Response<T> {
    inner: Result<T, ApiError>,
    url: Url,
    request_id: Option<String>,
    project_id: Option<String>,
    status_code: StatusCode,
    headers: http::HeaderMap,
}

impl<T> Response<T> {
    pub fn into_inner(self) -> Result<T, ApiError> {
        self.inner
    }

    pub fn inner(&self) -> &Result<T, ApiError> {
        &self.inner
    }

    pub fn request_id(&self) -> &Option<String> {
        &self.request_id
    }

    pub fn project_id(&self) -> &Option<String> {
        &self.project_id
    }

    pub fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }

    pub fn status_code(&self) -> http::StatusCode {
        self.status_code
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn is_err(&self) -> bool {
        self.inner.is_err()
    }

    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
}

impl<T> Response<T>
where
    T: DeserializeOwned,
{
    pub(crate) async fn from_raw_response(
        raw: reqwest::Response,
    ) -> Result<Self, crate::Error> {
        let url = raw.url().clone();
        let status_code = raw.status();
        let headers = raw.headers().clone();
        let project_id = headers
            .get(PROJECT_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());
        let request_id = headers
            .get(REQUEST_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let body = raw.text().await?;

        let inner = if status_code.is_success() {
            Ok(serde_json::from_str(&body)?)
        } else {
            // Attempt to parse the error as json
            let error_body: Result<ApiErrorBody, serde_json::Error> =
                serde_json::from_str(&body);
            match error_body {
                | Ok(error_body) => {
                    Err(ApiError {
                        status_code,
                        message: error_body.message,
                        params: error_body.params,
                    })
                }
                | Err(e) => {
                    warn!(
                        "Response error body is not json. Error: {}. Body: {}",
                        e, body
                    );
                    Err(ApiError {
                        status_code,
                        message: body,
                        params: None,
                    })
                }
            }
        };

        Ok(Self {
            inner,
            url,
            project_id,
            request_id,
            status_code,
            headers,
        })
    }
}
