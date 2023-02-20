mod handler;
mod ops;
mod sched;

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
