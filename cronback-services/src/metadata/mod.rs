mod config;
mod db_model;
mod handler;
mod metadata_store;
mod migration;

use async_trait::async_trait;
use lib::prelude::*;
use lib::{netutils, service};
use metadata_store::MetadataStore;
use proto::metadata_svc::metadata_svc_server::MetadataSvcServer;
use tracing::info;

use self::config::MetadataSvcConfig;

/// The primary service data type for the service.
#[derive(Clone)]
pub struct MetadataService;

#[async_trait]
impl CronbackService for MetadataService {
    type Migrator = migration::Migrator;
    type ServiceConfig = MetadataSvcConfig;

    const DEFAULT_CONFIG_TOML: &'static str = include_str!("config.toml");
    const ROLE: &'static str = "metadata";

    #[tracing::instrument(skip_all, fields(service = context.service_name()))]
    async fn serve(
        mut context: ServiceContext<Self>,
        db: Database,
    ) -> anyhow::Result<()> {
        let svc_config = context.service_config();
        let addr =
            netutils::parse_addr(&svc_config.address, svc_config.port).unwrap();

        let store = MetadataStore::new(db);

        let handler = handler::MetadataSvcHandler::new(context.clone(), store);
        let svc = MetadataSvcServer::new(handler);

        // grpc server
        info!("Starting Metadata service on {:?}", addr);

        // The stack of middleware that our service will be wrapped in
        service::grpc_serve_tcp(
            &mut context,
            addr,
            svc,
            svc_config.request_processing_timeout_s,
        )
        .await;

        Ok(())
    }
}
