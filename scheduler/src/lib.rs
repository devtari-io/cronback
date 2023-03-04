pub(crate) mod error;
pub(crate) mod handler;
pub(crate) mod sched;

use std::sync::Arc;

use handler::SchedulerAPIHandler;
use proto::scheduler_proto::scheduler_server::SchedulerServer;

use sched::event_scheduler::EventScheduler;
use shared::netutils;
use shared::service;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_scheduler_server(mut context: service::ServiceContext) {
    let config = context.load_config();
    let event_scheduler = Arc::new(EventScheduler::new(context.clone()));

    let addr =
        netutils::parse_addr(&config.scheduler.address, config.scheduler.port)
            .unwrap();
    event_scheduler.start();
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

    event_scheduler.shutdown();
}

pub mod test_helpers {
    use std::future::Future;
    use std::sync::Arc;

    use tempfile::NamedTempFile;
    use tokio::net::{UnixListener, UnixStream};
    use tokio_stream::wrappers::UnixListenerStream;
    use tonic::transport::{Channel, Endpoint, Server, Uri};
    use tower::service_fn;

    use crate::handler::SchedulerAPIHandler;
    use crate::sched::event_scheduler::EventScheduler;
    use proto::scheduler_proto::scheduler_client::SchedulerClient;
    use proto::scheduler_proto::scheduler_server::SchedulerServer;
    use shared::service::ServiceContext;

    pub async fn test_server_and_client(
        context: ServiceContext,
    ) -> (impl Future<Output = ()>, SchedulerClient<Channel>) {
        let socket = NamedTempFile::new().unwrap();
        let socket = Arc::new(socket.into_temp_path());
        std::fs::remove_file(&*socket).unwrap();

        let uds = UnixListener::bind(&*socket).unwrap();
        let stream = UnixListenerStream::new(uds);

        let event_scheduler = Arc::new(EventScheduler::new(context.clone()));
        event_scheduler.start();
        let handler =
            SchedulerAPIHandler::new(context.clone(), event_scheduler.clone());
        let svc = SchedulerServer::new(handler);

        let serve_future = async move {
            let result = Server::builder()
                .add_service(svc)
                .serve_with_incoming(stream)
                .await;
            event_scheduler.shutdown();
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