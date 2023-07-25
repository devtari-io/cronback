use std::task::{Context, Poll};
use std::time::Instant;

use hyper::Body;
use metrics::{histogram, increment_counter};
use tonic::body::BoxBody;
use tower::{Layer, Service};

use crate::consts::{PROJECT_ID_HEADER, REQUEST_ID_HEADER};
use crate::model::{ModelId, ValidShardedId};
use crate::types::{ProjectId, RequestId};

#[derive(Debug, Clone, Default)]
pub struct CronbackRpcMiddleware {
    /// Sets the label "service" in emitted metrics
    service_name: String,
}

impl CronbackRpcMiddleware {
    pub fn new(service_name: &str) -> CronbackRpcMiddleware {
        CronbackRpcMiddleware {
            service_name: service_name.into(),
        }
    }
}

impl<S> Layer<S> for CronbackRpcMiddleware {
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

        // Do we have a x-cronback-request-id header? Only used in grpc
        // services. The api-server will set the request-id header with
        // a random value.
        if let Some(cronback_request_id) = req.headers().get(REQUEST_ID_HEADER)
        {
            // If so, set the request id to the value of the header
            let cronback_request_id = cronback_request_id.to_str().unwrap();
            let cronback_request_id =
                RequestId::from(cronback_request_id.to_owned());
            req.extensions_mut().insert(cronback_request_id);
        }

        // Do we have a x-cronback-project-id header?
        // If project-id is set, it must be valid. We store the result in
        // extensions.
        if let Some(project_id) = req.headers().get(PROJECT_ID_HEADER) {
            // If so, set the project id to the value of the header
            let project_id = project_id.to_str().unwrap();
            let maybe_project_id =
                ProjectId::from(project_id.to_owned()).validated();
            req.extensions_mut().insert(maybe_project_id);
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
            let mut response = inner.call(req).await?;
            let latency_s = (Instant::now() - start).as_secs_f64();
            histogram!(
                "rpc.duration_seconds",
                latency_s,
                "service" => service_name.clone(),
                "endpoint" => endpoint.clone(),
            );

            // Inject request_id into response headers
            if let Some(request_id) =
                response.extensions().get::<RequestId>().cloned()
            {
                response.headers_mut().insert(
                    REQUEST_ID_HEADER,
                    request_id.to_string().parse().unwrap(),
                );
            }

            // Inject project_id into response headers

            if let Some(project_id) = response
                .extensions()
                .get::<ValidShardedId<ProjectId>>()
                .cloned()
            {
                response.headers_mut().insert(
                    PROJECT_ID_HEADER,
                    project_id.to_string().parse().unwrap(),
                );
            }

            Ok(response)
        })
    }
}
