mod attempt_store;
mod config;
mod db_model;
mod dispatch_manager;
mod handler;
mod migration;
mod retry;
mod run_store;
mod webhook_action;

use async_trait::async_trait;
use attempt_store::AttemptStore;
use dispatch_manager::DispatchManager;
use lib::prelude::*;
use lib::{netutils, service};
use metrics::{describe_counter, describe_gauge, Unit};
use proto::dispatcher_svc::dispatcher_svc_server::DispatcherSvcServer;
use run_store::RunStore;
use tracing::info;

use self::config::DispatcherSvcConfig;

/// The primary service data type for the service.
#[derive(Clone)]
pub struct DispatcherService;

#[async_trait]
impl CronbackService for DispatcherService {
    type Migrator = migration::Migrator;
    type ServiceConfig = DispatcherSvcConfig;

    const DEFAULT_CONFIG_TOML: &'static str = include_str!("config.toml");
    const ROLE: &'static str = "dispatcher";

    fn install_telemetry() {
        describe_counter!(
            "dispatcher.runs_total",
            Unit::Count,
            "Total number of runs by the dispatcher"
        );
        describe_counter!(
            "dispatcher.attempts_total",
            Unit::Count,
            "Total number of attempts attempted by the dispatcher"
        );

        describe_gauge!(
            "dispatcher.inflight_runs_total",
            Unit::Count,
            "Total number of inflight runs in the dispatcher"
        );
    }

    #[tracing::instrument(skip_all, fields(service = context.service_name()))]
    async fn serve(
        mut context: ServiceContext<Self>,
        db: Database,
    ) -> anyhow::Result<()> {
        let svc_config = context.service_config();
        let addr =
            netutils::parse_addr(&svc_config.address, svc_config.port).unwrap();

        let attempt_store = AttemptStore::new(db.clone());

        let run_store = RunStore::new(db);

        let dispatch_manager = DispatchManager::new(
            svc_config.cell_id,
            run_store.clone(),
            attempt_store,
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
            svc_config.request_processing_timeout_s,
        )
        .await;

        Ok(())
    }
}
