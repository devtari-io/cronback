mod dispatch_manager;
mod emits;
mod handler;
mod retry;
mod validators;

use proto::dispatcher_proto::dispatcher_server::DispatcherServer;
use tracing::info;

use shared::netutils;
use shared::service;

use crate::dispatch_manager::DispatchManager;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_dispatcher_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();
    let addr = netutils::parse_addr(
        &config.dispatcher.address,
        config.dispatcher.port,
    )
    .unwrap();

    let dispatch_manager = DispatchManager::create_and_start(context.clone());
    let handler =
        handler::DispatcherAPIHandler::new(context.clone(), dispatch_manager);
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

    Ok(())
}
