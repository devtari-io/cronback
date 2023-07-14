use std::task::{Context, Poll};
use std::time::Instant;

use hyper::Body;
use metrics::{histogram, increment_counter};
use tonic::body::BoxBody;
use tower::{Layer, Service};

use crate::types::RequestId;

#[derive(Debug, Clone, Default)]
pub struct TelemetryMiddleware {
    /// Sets the label "service" in emitted metrics
    service_name: String,
}

impl TelemetryMiddleware {
    pub fn new(service_name: &str) -> TelemetryMiddleware {
        TelemetryMiddleware {
            service_name: service_name.into(),
        }
    }
}

impl<S> Layer<S> for TelemetryMiddleware {
    type Service = InnerMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        InnerMiddleware::new(&self.service_name, service)
    }
}

#[derive(Debug, Clone)]
pub struct InnerMiddleware<S> {
    inner: S,
    service_name: String,
}

impl<S> InnerMiddleware<S> {
    pub fn new(service_name: &str, inner: S) -> Self {
        InnerMiddleware {
            inner,
            service_name: service_name.to_owned(),
        }
    }
}

impl<S> Service<hyper::Request<Body>> for InnerMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Error = S::Error;
    type Future = futures::future::BoxFuture<
        'static,
        Result<Self::Response, Self::Error>,
    >;
    type Response = S::Response;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses
        // `tower::buffer::Buffer`. See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        // Do we have a cronback-request-id header?
        if let Some(cronback_request_id) =
            req.headers().get("cronback-request-id")
        {
            // If so, set the request id to the value of the header
            let cronback_request_id = cronback_request_id.to_str().unwrap();
            let cronback_request_id =
                RequestId::from(cronback_request_id.to_owned());
            req.extensions_mut().insert(cronback_request_id);
        }

        // Removes the leading '/' in the path.
        let endpoint = req.uri().path()[1..].to_owned();
        let service_name = self.service_name.clone();
        let start = Instant::now();
        increment_counter!(
            "rpc.requests_total",
            "service" => service_name.clone(),
            "endpoint" => endpoint.clone()
        );
        Box::pin(async move {
            let response = inner.call(req).await?;
            let latency_s = (Instant::now() - start).as_secs_f64();
            histogram!(
                "rpc.duration_seconds",
                latency_s,
                "service" => service_name.clone(),
                "endpoint" => endpoint.clone(),
            );

            Ok(response)
        })
    }
}
