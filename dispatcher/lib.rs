#![feature(ip, assert_matches)]

mod handler;
mod validators;
mod webhook;

use proto::FILE_DESCRIPTOR_SET;
use shared::rpc_middleware::TelemetryMiddleware;
use tonic_reflection::server::Builder;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use proto::dispatcher_proto::dispatcher_server::DispatcherServer;
use tonic::transport::Server;
use tracing::info;

use shared::config::ConfigLoader;
use shared::netutils;

pub async fn start_dispatcher(config_loader: Arc<ConfigLoader>) -> Result<()> {
    let config = config_loader.load()?;
    let addr = netutils::parse_addr(&config.dispatcher.address, config.dispatcher.port)?;
    let handler = handler::DispatcherAPIHandler {
        config_loader: config_loader.clone(),
    };
    let svc = DispatcherServer::new(handler);

    // The stack of middleware that our service will be wrapped in
    let telemetry_middleware = tower::ServiceBuilder::new()
        // Apply our own middleware
        .layer(TelemetryMiddleware::new("dispatcher"))
        .into_inner();

    // grpc server
    info!("Starting Dispatcher on {:?}", addr);

    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    Server::builder()
        .timeout(Duration::from_secs(
            config.dispatcher.request_processing_timeout_s,
        ))
        .layer(telemetry_middleware)
        .add_service(reflection_service)
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}
