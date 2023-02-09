use std::sync::Arc;

use anyhow::Result;
use proto::dispatcher_proto::{
    dispatcher_server::{Dispatcher, DispatcherServer},
    EchoRequest, EchoResponse,
};
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

use shared::config::ConfigLoader;
use shared::netutils;

pub async fn start_dispatcher(config_loader: Arc<ConfigLoader>) -> Result<()> {
    let config = config_loader.load()?;
    let addr = netutils::parse_addr(config.dispatcher.address, config.dispatcher.port)?;
    info!("Starting Dispatcher on {:?}", addr);
    Server::builder()
        .add_service(DispatcherServer::new(MyEcho::default()))
        .serve(addr)
        .await?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct MyEcho {}

#[tonic::async_trait]
impl Dispatcher for MyEcho {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        println!("Got a request: {request:?}");

        let reply = EchoResponse {
            name: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}
