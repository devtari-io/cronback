mod dispatch_manager;
mod emits;
mod handler;
mod retry;

use std::sync::Arc;

use lib::database::attempt_log_store::{AttemptLogStore, SqlAttemptLogStore};
use lib::database::invocation_store::{InvocationStore, SqlInvocationStore};
use lib::database::Database;
use lib::{netutils, service};
use proto::dispatcher_proto::dispatcher_server::DispatcherServer;
use tracing::info;

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

    let db = Database::connect(&config.dispatcher.database_uri).await?;
    let attempt_store: Arc<dyn AttemptLogStore + Send + Sync> = Arc::new({
        let s = SqlAttemptLogStore::new(db.clone());
        s.prepare().await?;
        s
    });

    let invocation_store: Arc<dyn InvocationStore + Send + Sync> = Arc::new({
        let s = SqlInvocationStore::new(db);
        s.prepare().await?;
        s
    });

    let dispatch_manager =
        DispatchManager::new(invocation_store, attempt_store);
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
