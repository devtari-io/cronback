pub(crate) mod error;
pub(crate) mod handler;
pub(crate) mod sched;

use std::sync::Arc;
use std::time::Duration;

use handler::SchedulerAPIHandler;
use proto::scheduler_proto::scheduler_server::SchedulerServer;
use sched::event_scheduler::EventScheduler;
use shared::database::trigger_store::SqlTriggerStore;
use shared::database::SqliteDatabase;
use shared::grpc_client_provider::DispatcherClientProvider;
use shared::{netutils, service};

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_scheduler_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();

    let db = SqliteDatabase::connect(&config.scheduler.database_uri).await?;
    let trigger_store = SqlTriggerStore::create(db).await?;

    let dispatcher_client_provider = Arc::new(DispatcherClientProvider::new(
        config.scheduler.dispatcher_uri.clone(),
    ));

    let event_scheduler = Arc::new(EventScheduler::new(
        context.clone(),
        Box::new(trigger_store),
        dispatcher_client_provider,
    ));

    let addr =
        netutils::parse_addr(&config.scheduler.address, config.scheduler.port)
            .unwrap();
    event_scheduler.start().await?;

    let async_es = event_scheduler.clone();
    let db_flush_s = config.scheduler.db_flush_s;
    tokio::spawn(async move {
        let sleep = Duration::from_secs(db_flush_s);
        loop {
            tokio::time::sleep(sleep).await;
            async_es.perform_checkpoint().await;
        }
    });

    let handler =
        SchedulerAPIHandler::new(context.clone(), event_scheduler.clone());
    let svc = SchedulerServer::new(handler);

    // grpc server
    service::grpc_serve(
        &mut context,
        addr,
        svc,
        config.scheduler.request_processing_timeout_s,
    )
    .await;

    event_scheduler.shutdown().await;
    Ok(())
}

pub mod test_helpers {
    use std::future::Future;
    use std::sync::Arc;

    use proto::scheduler_proto::scheduler_client::SchedulerClient;
    use proto::scheduler_proto::scheduler_server::SchedulerServer;
    use shared::database::trigger_store::SqlTriggerStore;
    use shared::database::SqliteDatabase;
    use shared::grpc_client_provider::DispatcherClientProvider;
    use shared::service::ServiceContext;
    use tempfile::NamedTempFile;
    use tokio::net::{UnixListener, UnixStream};
    use tokio_stream::wrappers::UnixListenerStream;
    use tonic::transport::{Channel, Endpoint, Server, Uri};
    use tower::service_fn;

    use crate::handler::SchedulerAPIHandler;
    use crate::sched::event_scheduler::EventScheduler;

    pub async fn test_server_and_client(
        context: ServiceContext,
    ) -> (impl Future<Output = ()>, SchedulerClient<Channel>) {
        let socket = NamedTempFile::new().unwrap();
        let socket = Arc::new(socket.into_temp_path());
        std::fs::remove_file(&*socket).unwrap();

        let uds = UnixListener::bind(&*socket).unwrap();
        let stream = UnixListenerStream::new(uds);

        let dispatcher_client_provider =
            Arc::new(DispatcherClientProvider::new(
                context.load_config().scheduler.dispatcher_uri,
            ));

        let db = SqliteDatabase::in_memory().await.unwrap();
        let trigger_store = SqlTriggerStore::create(db).await.unwrap();
        let event_scheduler = Arc::new(EventScheduler::new(
            context.clone(),
            Box::new(trigger_store),
            dispatcher_client_provider,
        ));
        event_scheduler.start().await.unwrap();

        let handler =
            SchedulerAPIHandler::new(context.clone(), event_scheduler.clone());
        let svc = SchedulerServer::new(handler);

        let serve_future = async move {
            let result = Server::builder()
                .add_service(svc)
                .serve_with_incoming(stream)
                .await;
            event_scheduler.shutdown().await;
            // Validate that server is running fine...
            assert!(result.is_ok());
        };

        let socket = Arc::clone(&socket);
        // Connect to the server over a Unix socket
        // The URL will be ignored.
        let channel = Endpoint::try_from("http://example.url")
            .unwrap()
            .connect_with_connector(service_fn(move |_: Uri| {
                let socket = Arc::clone(&socket);
                async move { UnixStream::connect(&*socket).await }
            }))
            .await
            .unwrap();

        let client = SchedulerClient::new(channel);

        (serve_future, client)
    }
}
