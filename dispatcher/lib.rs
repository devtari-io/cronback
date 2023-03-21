pub(crate) mod attempt_log_store;
mod dispatch_manager;
mod emits;
mod handler;
pub(crate) mod invocation_store;
mod retry;
mod validators;

use std::sync::Arc;

use proto::dispatcher_proto::dispatcher_server::DispatcherServer;
use shared::database::SqliteDatabase;
use shared::{netutils, service};
use tracing::info;

use crate::attempt_log_store::{AttemptLogStore, SqlAttemptLogStore};
use crate::dispatch_manager::DispatchManager;
use crate::invocation_store::{InvocationStore, SqlInvocationStore};

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

    let db = SqliteDatabase::connect(&config.dispatcher.database_uri).await?;
    let attempt_store: Arc<dyn AttemptLogStore + Send + Sync> =
        Arc::new(SqlAttemptLogStore::create(db.clone()).await?);

    let invocation_store: Arc<dyn InvocationStore + Send + Sync> =
        Arc::new(SqlInvocationStore::create(db).await?);

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
