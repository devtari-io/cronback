mod config;
pub(crate) mod db_model;
pub(crate) mod error;
pub(crate) mod handler;
mod migration;
pub(crate) mod spinner;
pub(crate) mod trigger_store;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use handler::SchedulerSvcHandler;
use lib::prelude::*;
use lib::service::{CronbackService, ServiceContext};
use lib::{netutils, service, GrpcClientProvider};
use metrics::{describe_gauge, describe_histogram, Unit};
use proto::scheduler_svc::scheduler_svc_server::SchedulerSvcServer;
use spinner::controller::SpinnerController;
use trigger_store::TriggerStore;

use self::config::SchedulerSvcConfig;

/// The primary service data type for the service.
#[derive(Clone)]
pub struct SchedulerService;

#[async_trait]
impl CronbackService for SchedulerService {
    type Migrator = migration::Migrator;
    type ServiceConfig = SchedulerSvcConfig;

    const DEFAULT_CONFIG_TOML: &'static str = include_str!("config.toml");
    const ROLE: &'static str = "scheduler";

    fn install_telemetry() {
        describe_histogram!(
            "spinner.yield_duration_ms",
            Unit::Milliseconds,
            "The time where the spinner gets to sleep until next tick"
        );
        describe_histogram!(
            "spinner.dispatch_lag_seconds",
            Unit::Seconds,
            "How many seconds the spinner is lagging from trigger ticks"
        );
        describe_gauge!(
            "spinner.active_triggers_total",
            Unit::Count,
            "How many active triggers are loaded into the spinner"
        );
    }

    #[tracing::instrument(skip_all, fields(service = context.service_name()))]
    async fn serve(
        mut context: ServiceContext<Self>,
        db: Database,
    ) -> anyhow::Result<()> {
        let config = context.service_config();

        let trigger_store = TriggerStore::new(db);

        let dispatcher_clients =
            Arc::new(GrpcClientProvider::new(context.config().clone()));

        let controller = Arc::new(SpinnerController::new(
            context.clone(),
            trigger_store,
            dispatcher_clients,
        ));

        let addr = netutils::parse_addr(&config.address, config.port).unwrap();
        controller.start().await?;

        let async_es = controller.clone();
        let db_flush_s = config.db_flush_s;
        tokio::spawn(async move {
            let sleep = Duration::from_secs(db_flush_s);
            loop {
                tokio::time::sleep(sleep).await;
                async_es.perform_checkpoint().await;
            }
        });

        let handler =
            SchedulerSvcHandler::new(context.clone(), controller.clone());
        let svc = SchedulerSvcServer::new(handler);

        // grpc server
        service::grpc_serve_tcp(
            &mut context,
            addr,
            svc,
            config.request_processing_timeout_s,
        )
        .await;

        controller.shutdown().await;
        Ok(())
    }
}

pub mod test_helpers {
    use std::sync::Arc;

    use lib::clients::ScopedSchedulerSvcClient;
    use lib::grpc_test_helpers::TestGrpcClientProvider;
    use lib::service::{self, ServiceContext};
    use lib::GrpcClientProvider;
    use tempfile::NamedTempFile;
    use tokio::task::JoinHandle;

    use super::*;

    pub async fn test_server_and_client(
        mut context: ServiceContext<SchedulerService>,
    ) -> (
        JoinHandle<()>,
        TestGrpcClientProvider<ScopedSchedulerSvcClient>,
    ) {
        let socket = NamedTempFile::new().unwrap();
        let socket = Arc::new(socket.into_temp_path());
        std::fs::remove_file(&*socket).unwrap();

        let dispatcher_client_provider =
            Arc::new(GrpcClientProvider::new(context.config().clone()));

        let db = SchedulerService::in_memory_database().await.unwrap();

        let trigger_store = TriggerStore::new(db);
        let controller = Arc::new(SpinnerController::new(
            context.clone(),
            trigger_store,
            dispatcher_client_provider,
        ));
        controller.start().await.unwrap();

        let handler =
            SchedulerSvcHandler::new(context.clone(), controller.clone());
        let svc = SchedulerSvcServer::new(handler);

        let cloned_socket = Arc::clone(&socket);

        let serve_future = tokio::spawn(async move {
            let request_processing_timeout_s = 3;
            service::grpc_serve_unix(
                &mut context,
                &*cloned_socket,
                svc,
                request_processing_timeout_s,
            )
            .await;
        });

        // Give the server time to start.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        let client_provider = TestGrpcClientProvider::new_single_shard(socket);

        (serve_future, client_provider)
    }
}
