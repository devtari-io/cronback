mod db_model;
mod handler;
mod metadata_store;

use std::sync::Arc;

use lib::database::Database;
use lib::{netutils, service};
use metadata_store::{MetadataStore, SqlMetadataStore};
use proto::metadata_svc::metadata_svc_server::MetadataSvcServer;
use tracing::info;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_metadata_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();
    let addr =
        netutils::parse_addr(&config.metadata.address, config.metadata.port)
            .unwrap();

    let db = Database::connect(&config.metadata.database_uri).await?;
    db.migrate().await?;

    let store: Arc<dyn MetadataStore + Send + Sync> =
        Arc::new(SqlMetadataStore::new(db));

    let handler = handler::MetadataSvcHandler::new(context.clone(), store);
    let svc = MetadataSvcServer::new(handler);

    // grpc server
    info!("Starting Metadata service on {:?}", addr);

    // The stack of middleware that our service will be wrapped in
    service::grpc_serve_tcp(
        &mut context,
        addr,
        svc,
        config.metadata.request_processing_timeout_s,
    )
    .await;

    Ok(())
}
