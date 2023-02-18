mod handler;
mod ops;
mod sched;

use proto::scheduler_proto::scheduler_server::SchedulerServer;

use sched::spinner::Spinner;
use shared::netutils;
use shared::service;

pub async fn start_scheduler_server(mut context: service::ServiceContext) {
    let config = context.load_config();
    let spinner = Spinner::new().start();

    let addr = netutils::parse_addr(&config.scheduler.address, config.scheduler.port).unwrap();
    let handler = handler::SchedulerAPIHandler {
        config_loader: context.config_loader(),
    };
    let svc = SchedulerServer::new(handler);

    // grpc server
    service::grpc_serve(
        &mut context,
        addr,
        svc,
        config.scheduler.request_processing_timeout_s,
    )
    .await;
    spinner.shutdown();
}
