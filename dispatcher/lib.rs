mod actions;
mod dispatch_manager;
mod handler;
mod retry;

use std::sync::Arc;

use lib::database::attempt_log_store::{AttemptLogStore, SqlAttemptLogStore};
use lib::database::run_store::{RunStore, SqlRunStore};
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
    let attempt_store: Arc<dyn AttemptLogStore + Send + Sync> =
        Arc::new(SqlAttemptLogStore::new(db.clone()));

    let run_store: Arc<dyn RunStore + Send + Sync> =
        Arc::new(SqlRunStore::new(db));

    let dispatch_manager =
        DispatchManager::new(run_store.clone(), attempt_store.clone());
    let handler = handler::DispatcherAPIHandler::new(
        context.clone(),
        dispatch_manager,
        run_store,
        attempt_store,
    );
    let svc = DispatcherServer::new(handler);

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
