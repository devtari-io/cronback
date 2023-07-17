mod db_model;
mod handler;
mod project_store;

use std::sync::Arc;

use lib::database::Database;
use lib::{netutils, service};
use project_store::{ProjectStore, SqlProjectStore};
use proto::project_svc_proto::project_service_server::ProjectServiceServer;
use tracing::info;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_project_store_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();
    let addr = netutils::parse_addr(
        &config.project_srv.address,
        config.project_srv.port,
    )
    .unwrap();

    let db = Database::connect(&config.project_srv.database_uri).await?;
    db.migrate().await?;

    let project_store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(SqlProjectStore::new(db));

    let handler =
        handler::ProjectServiceAPIHandler::new(context.clone(), project_store);
    let svc = ProjectServiceServer::new(handler);

    // grpc server
    info!("Starting ProjectSrv on {:?}", addr);

    // The stack of middleware that our service will be wrapped in
    service::grpc_serve_tcp(
        &mut context,
        addr,
        svc,
        config.project_srv.request_processing_timeout_s,
    )
    .await;

    Ok(())
}
