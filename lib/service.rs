use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper::{Body, Request, Response};
use proto::FILE_DESCRIPTOR_SET;
use tonic::body::BoxBody;
use tonic::transport::{NamedService, Server};
use tonic_reflection::server::Builder;
use tower::Service;
use tracing::{error, info};

use crate::config::{Config, ConfigLoader};
use crate::rpc_middleware::TelemetryMiddleware;
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

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn grpc_serve<S>(
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
    // The stack of middleware that our service will be wrapped in
    let telemetry_middleware = tower::ServiceBuilder::new()
        // Apply our own middleware
        .layer(TelemetryMiddleware::new(context.service_name()))
        .into_inner();

    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    // grpc Server
    info!("Starting '{}' on {:?}", context.service_name(), addr);
    if let Err(e) = Server::builder()
        .timeout(Duration::from_secs(timeout))
        .layer(telemetry_middleware)
        .add_service(reflection_service)
        .add_service(svc)
        .serve_with_shutdown(addr, context.recv_shutdown_signal())
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
