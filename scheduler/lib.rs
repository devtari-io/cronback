mod handler;

use shared::rpc_middleware::TelemetryMiddleware;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use proto::scheduler_proto::scheduler_server::SchedulerServer;
use tonic::transport::Server;
use tracing::info;

use shared::config::ConfigLoader;
use shared::netutils;

pub async fn start_scheduler(config_loader: Arc<ConfigLoader>) -> Result<()> {
    let config = config_loader.load()?;
    let addr = netutils::parse_addr(&config.scheduler.address, config.scheduler.port)?;
    let handler = handler::SchedulerAPIHandler {
        config_loader: config_loader.clone(),
    };
    let svc = SchedulerServer::new(handler);

    // The stack of middleware that our service will be wrapped in
    let telemetry_middleware = tower::ServiceBuilder::new()
        // Apply our own middleware
        .layer(TelemetryMiddleware::new("scheduler"))
        .into_inner();

    // grpc server
    info!("Starting Scheduler on {:?}", addr);
    Server::builder()
        .timeout(Duration::from_secs(
            config.scheduler.request_processing_timeout_s,
        ))
        .layer(telemetry_middleware)
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}
