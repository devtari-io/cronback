mod actions;
mod dispatch_manager;
mod handler;
mod retry;

use std::sync::Arc;

use dispatch_manager::DispatchManager;
use lib::database::attempt_log_store::{AttemptLogStore, SqlAttemptLogStore};
use lib::database::run_store::{RunStore, SqlRunStore};
use lib::database::Database;
use lib::{netutils, service};
use proto::dispatcher_svc::dispatcher_svc_server::DispatcherSvcServer;
use tracing::info;

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
    db.migrate().await?;
    let attempt_store: Arc<dyn AttemptLogStore + Send + Sync> =
        Arc::new(SqlAttemptLogStore::new(db.clone()));

    let run_store: Arc<dyn RunStore + Send + Sync> =
        Arc::new(SqlRunStore::new(db));

    let dispatch_manager = DispatchManager::new(
        config.dispatcher.cell_id,
        run_store.clone(),
        attempt_store.clone(),
    );
    dispatch_manager.start().await?;

    let handler = handler::DispatcherSvcHandler::new(
        context.clone(),
        dispatch_manager,
        run_store,
    );
    let svc = DispatcherSvcServer::new(handler);

    // grpc server
    info!("Starting Dispatcher on {:?}", addr);

    // The stack of middleware that our service will be wrapped in
    service::grpc_serve_tcp(
        &mut context,
        addr,
        svc,
        config.dispatcher.request_processing_timeout_s,
    )
    .await;

    Ok(())
}
