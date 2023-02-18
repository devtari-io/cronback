mod handler;
mod validators;
mod webhook;

use proto::dispatcher_proto::dispatcher_server::DispatcherServer;
use tracing::info;

use shared::netutils;
use shared::service;

pub async fn start_dispatcher_server(mut context: service::ServiceContext) {
    let config = context.load_config();
    let addr = netutils::parse_addr(&config.dispatcher.address, config.dispatcher.port).unwrap();
    let handler = handler::DispatcherAPIHandler {
        config_loader: context.config_loader(),
    };
    let svc = DispatcherServer::new(handler);

    // grpc server
    info!("Starting Dispatcher on {:?}", addr);

    // The stack of middleware that our service will be wrapped in
    service::grpc_serve(
        &mut context,
        addr,
        svc,
        config.dispatcher.request_processing_timeout_s,
    )
    .await;
}
