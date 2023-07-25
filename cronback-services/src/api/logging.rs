use std::borrow::Cow;
use std::fmt::Display;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use hyper::header::USER_AGENT;
use lib::prelude::*;
use lib::Config;
use tower_http::trace::MakeSpan;
use tracing::{error_span, info};

use super::auth::API_KEY_PREFIX;

#[derive(Clone, Debug)]
pub struct ApiMakeSpan {
    service_name: String,
}

impl ApiMakeSpan {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

impl<B> MakeSpan<B> for ApiMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        // Do we have a cronback user agent?
        let user_agent = request.headers().get(USER_AGENT);
        // We get the request_id from extensions
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .map(ToString::to_string);
        error_span!(
            target: "request_response_tracing_metadata",
            "http_request",
             // Then we put request_id into the span
             service = %self.service_name,
             request_id = %request_id.unwrap_or_default(),
             method = %request.method(),
             uri = %request.uri(),
             version = ?request.version(),
             user_agent = ?user_agent,
        )
    }
}

/// Log the request and response bodies for debugging purposes. Note this will
/// be logged within a span that already contains more information about the
/// request (e.g. method, uri, project_id, request_id, etc).
///
/// Mostly inspired from: https://github.com/tokio-rs/axum/blob/main/examples/consume-body-in-extractor-or-middleware/src/main.rs
pub async fn trace_request_response(
    State(config): State<Arc<Config>>,
    mut request: Request<axum::body::Body>,
    next: Next<axum::body::Body>,
) -> Result<impl IntoResponse, Response> {
    let config = &config.api;

    if config.log_request_body {
        // Break the request into parts to be able to read the body
        let (parts, body) = request.into_parts();

        let bytes = buffer_and_print_body(body, "Got request", None).await?;

        // Re-assemble the request back
        request = Request::from_parts(parts, axum::body::Body::from(bytes));
    }

    // Invoke the next middleware and wait for the response
    let resp = next.run(request).await;

    if config.log_response_body {
        // Break the response into parts to be able to read the body
        let status = resp.status().as_u16();
        let (parts, body) = resp.into_parts();

        let bytes =
            buffer_and_print_body(body, "Sent response", Some(status)).await?;

        // Re-assemble the response back
        return Ok(Response::from_parts(parts, axum::body::Body::from(bytes))
            .into_response());
    }

    Ok(resp)
}

async fn buffer_and_print_body<B>(
    body: B,
    msg: &str,
    status: Option<u16>,
) -> Result<axum::body::Bytes, Response>
where
    B: axum::body::HttpBody,
    <B as axum::body::HttpBody>::Error: Display,
{
    let bytes = hyper::body::to_bytes(body).await.map_err(|err| {
        (StatusCode::BAD_REQUEST, err.to_string()).into_response()
    })?;
    let mut body_str = String::from_utf8_lossy(&bytes);
    if body_str.find(API_KEY_PREFIX).is_some() {
        body_str = Cow::from("REDACTED");
    }

    // Log the response body
    info!(target: "request_response_tracing", body = %body_str, status, msg);

    Ok(bytes)
}
