use std::sync::Arc;

use anyhow::Result;
use proto::scheduler_proto::{
    scheduler_server::{Scheduler, SchedulerServer},
    EchoRequest, EchoResponse,
};
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

use shared::config::ConfigLoader;
use shared::netutils;

pub async fn start_scheduler(config_loader: Arc<ConfigLoader>) -> Result<()> {
    let config = config_loader.load()?;
    let addr = netutils::parse_addr(&config.scheduler.address, config.scheduler.port)?;
    info!("Starting Scheduler on {:?}", addr);
    Server::builder()
        .add_service(SchedulerServer::new(MyEcho::default()))
        .serve(addr)
        .await?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct MyEcho {}

#[tonic::async_trait]
impl Scheduler for MyEcho {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        println!("Got a request: {request:?}");

        let reply = EchoResponse {
            name: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}
