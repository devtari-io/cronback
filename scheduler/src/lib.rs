pub(crate) mod db_model;
pub(crate) mod error;
pub(crate) mod handler;
pub(crate) mod spinner;
pub(crate) mod trigger_store;

use std::sync::Arc;
use std::time::Duration;

use handler::SchedulerAPIHandler;
use lib::database::Database;
use lib::grpc_client_provider::GrpcClientProvider;
use lib::{netutils, service};
use proto::scheduler_proto::scheduler_server::SchedulerServer;

use crate::spinner::controller::SpinnerController;
use crate::trigger_store::SqlTriggerStore;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_scheduler_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();

    let db = Database::connect(&config.scheduler.database_uri).await?;
    db.migrate().await?;
    let trigger_store = SqlTriggerStore::new(db);

    let dispatcher_clients = Arc::new(GrpcClientProvider::new(context.clone()));

    let controller = Arc::new(SpinnerController::new(
        context.clone(),
        Box::new(trigger_store),
        dispatcher_clients,
    ));

    let addr =
        netutils::parse_addr(&config.scheduler.address, config.scheduler.port)
            .unwrap();
    controller.start().await?;

    let async_es = controller.clone();
    let db_flush_s = config.scheduler.db_flush_s;
    tokio::spawn(async move {
        let sleep = Duration::from_secs(db_flush_s);
        loop {
            tokio::time::sleep(sleep).await;
            async_es.perform_checkpoint().await;
        }
    });

    let handler = SchedulerAPIHandler::new(context.clone(), controller.clone());
    let svc = SchedulerServer::new(handler);

    // grpc server
    service::grpc_serve_tcp(
        &mut context,
        addr,
        svc,
        config.scheduler.request_processing_timeout_s,
    )
    .await;

    controller.shutdown().await;
    Ok(())
}

pub mod test_helpers {
    use std::sync::Arc;

    use lib::clients::scheduler_client::ScopedSchedulerClient;
    use lib::database::Database;
    use lib::grpc_client_provider::test_helpers::TestGrpcClientProvider;
    use lib::grpc_client_provider::GrpcClientProvider;
    use lib::service::{self, ServiceContext};
    use proto::scheduler_proto::scheduler_server::SchedulerServer;
    use tempfile::NamedTempFile;
    use tokio::task::JoinHandle;

    use crate::handler::SchedulerAPIHandler;
    use crate::spinner::controller::SpinnerController;
    use crate::trigger_store::SqlTriggerStore;

    pub async fn test_server_and_client(
        mut context: ServiceContext,
    ) -> (
        JoinHandle<()>,
        TestGrpcClientProvider<ScopedSchedulerClient>,
    ) {
        let socket = NamedTempFile::new().unwrap();
        let socket = Arc::new(socket.into_temp_path());
        std::fs::remove_file(&*socket).unwrap();

        let dispatcher_client_provider =
            Arc::new(GrpcClientProvider::new(context.clone()));

        let db = Database::in_memory().await.unwrap();
        let trigger_store = SqlTriggerStore::new(db);
        let controller = Arc::new(SpinnerController::new(
            context.clone(),
            Box::new(trigger_store),
            dispatcher_client_provider,
        ));
        controller.start().await.unwrap();

        let handler =
            SchedulerAPIHandler::new(context.clone(), controller.clone());
        let svc = SchedulerServer::new(handler);

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
