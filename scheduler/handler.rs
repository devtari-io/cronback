use std::{sync::Arc, time::Duration};

use proto::scheduler_proto::{scheduler_server::Scheduler, EchoRequest, EchoResponse};
use tonic::{Request, Response, Status};

use shared::config::ConfigLoader;
use tracing::info;

pub(crate) struct SchedulerAPIHandler {
    pub config_loader: Arc<ConfigLoader>,
}

#[tonic::async_trait]
impl Scheduler for SchedulerAPIHandler {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        info!("Got a request: {request:?}");

        tokio::time::sleep(Duration::from_millis(800)).await;
        let reply = EchoResponse {
            name: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}
