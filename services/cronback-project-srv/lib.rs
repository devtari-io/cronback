mod db_model;
mod handler;
mod project_store;

use std::sync::Arc;

use lib::database::Database;
use lib::{netutils, service};
use project_store::{ProjectStore, SqlProjectStore};
use proto::project_srv_proto::project_service_server::ProjectServiceServer;
use tracing::info;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_project_store_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();
    let addr = netutils::parse_addr(
        &config.project_store.address,
        config.project_store.port,
    )
    .unwrap();

    let db = Database::connect(&config.project_store.database_uri).await?;
    db.migrate().await?;

    let project_store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(SqlProjectStore::new(db));

    let handler =
        handler::ProjectServiceAPIHandler::new(context.clone(), project_store);
    let svc = ProjectServiceServer::new(handler);

    // grpc server
    info!("Starting ProjectStore on {:?}", addr);

    // The stack of middleware that our service will be wrapped in
    service::grpc_serve_tcp(
        &mut context,
        addr,
        svc,
        config.project_store.request_processing_timeout_s,
    )
    .await;

    Ok(())
}
