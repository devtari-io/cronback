use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use hyper::{Body, Request, Response};
use proto::FILE_DESCRIPTOR_SET;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::body::BoxBody;
use tonic::transport::server::{Connected, TcpIncoming};
use tonic::transport::{NamedService, Server};
use tonic_reflection::server::Builder;
use tower::Service;
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::{error, error_span, info, Id, Span};

use crate::config::{Config, ConfigLoader};
use crate::consts::{PARENT_SPAN_HEADER, PROJECT_ID_HEADER, REQUEST_ID_HEADER};
use crate::rpc_middleware::CronbackRpcMiddleware;
use crate::shutdown::Shutdown;

#[derive(Clone)]
pub struct ServiceContext {
    name: String,
    config_loader: Arc<ConfigLoader>,
    shutdown: Shutdown,
}

impl ServiceContext {
    pub fn new(
        name: String,
        config_loader: Arc<ConfigLoader>,
        shutdown: Shutdown,
    ) -> Self {
        Self {
            name,
            config_loader,
            shutdown,
        }
    }

    pub fn service_name(&self) -> &str {
        &self.name
    }

    pub fn get_config(&self) -> Config {
        self.config_loader.load().unwrap()
    }

    pub fn config_loader(&self) -> Arc<ConfigLoader> {
        self.config_loader.clone()
    }

    pub fn load_config(&self) -> Config {
        self.config_loader.load().unwrap()
    }

    /// Awaits the shutdown signal
    pub async fn recv_shutdown_signal(&mut self) {
        self.shutdown.recv().await
    }

    /// Causes all listeners to start the shutdown sequence.
    pub fn broadcast_shutdown(&mut self) {
        self.shutdown.broadcast_shutdown()
    }
}

#[derive(Clone, Debug)]
struct GrpcMakeSpan {
    service_name: String,
}
impl GrpcMakeSpan {
    fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

impl<B> MakeSpan<B> for GrpcMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let request_id = request
            .headers()
            .get(REQUEST_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let parent_span = request
            .headers()
            .get(PARENT_SPAN_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let project_id = request
            .headers()
            .get(PROJECT_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let span = error_span!(
            "grpc_request",
             service = %self.service_name,
             request_id = %request_id.unwrap_or_default(),
             project_id = %project_id.unwrap_or_default(),
             method = %request.method(),
             uri = %request.uri(),
             version = ?request.version(),
        );

        if let Some(parent_span) = parent_span {
            let id = Id::from_u64(parent_span.parse().unwrap());
            span.follows_from(id);
        }
        span
    }
}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn grpc_serve_tcp<S>(
    context: &mut ServiceContext,
    addr: SocketAddr,
    svc: S,
    timeout: u64,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    info!("Starting '{}' on {:?}", context.service_name(), addr);
    match TcpIncoming::new(addr, true, None) {
        | Ok(incoming) => {
            grpc_serve_incoming(context, svc, incoming, timeout).await
        }
        | Err(e) => {
            error!(
                "RPC service '{}' couldn't bind on address '{addr}', system \
                 will shutdown: {e}",
                context.service_name()
            );
            context.broadcast_shutdown();
        }
    };
}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn grpc_serve_unix<S, K>(
    context: &mut ServiceContext,
    socket: K,
    svc: S,
    timeout: u64,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    K: AsRef<Path>,
{
    info!(
        "Starting '{}' on {:?}",
        context.service_name(),
        socket.as_ref()
    );
    let uds = UnixListener::bind(socket).unwrap();
    let stream = UnixListenerStream::new(uds);
    grpc_serve_incoming(context, svc, stream, timeout).await
}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
async fn grpc_serve_incoming<S, K, IO, IE>(
    context: &mut ServiceContext,
    svc: S,
    incoming: K,
    timeout: u64,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    K: Stream<Item = Result<IO, IE>>,
    IO: AsyncRead + AsyncWrite + Connected + Unpin + Send + 'static,
    IO::ConnectInfo: Clone + Send + Sync + 'static,
    IE: Into<Box<dyn Error + Send + Sync>>,
{
    let svc_name = context.service_name().to_owned();
    // The stack of middleware that our service will be wrapped in
    let cronback_middleware = tower::ServiceBuilder::new()
        // Apply our own middleware
        .layer(
            TraceLayer::new_for_grpc()
                .make_span_with(GrpcMakeSpan::new(svc_name)),
        )
        .layer(CronbackRpcMiddleware::new(context.service_name()))
        .into_inner();

    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    // grpc Server
    if let Err(e) = Server::builder()
        .timeout(Duration::from_secs(timeout))
        .layer(cronback_middleware)
        .add_service(reflection_service)
        .add_service(svc)
        .serve_with_incoming_shutdown(incoming, context.recv_shutdown_signal())
        .await
    {
        error!(
            "RPC service '{}' failed to start and will trigger system \
             shutdown: {e}",
            context.service_name()
        );
        context.broadcast_shutdown()
    } else {
        info!("Service '{}' terminated", context.service_name());
    }
}
