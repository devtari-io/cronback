use std::time::Duration;

use tonic::{Request, Response, Status};
use tracing::info;

use proto::scheduler_proto::{
    scheduler_server::Scheduler, ExecuteTriggerRequest, ExecuteTriggerResponse,
    FindTriggersRequest, FindTriggersResponse, GetTriggerRequest, GetTriggerResponse,
    InstallTriggerRequest, InstallTriggerResponse, UpdateTriggerRequest, UpdateTriggerResponse,
};
use shared::service::ServiceContext;

pub(crate) struct SchedulerAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
}
impl SchedulerAPIHandler {
    pub(crate) fn new(context: ServiceContext) -> Self {
        Self { context }
    }
}

#[tonic::async_trait]
impl Scheduler for SchedulerAPIHandler {
    async fn install_trigger(
        &self,
        request: Request<InstallTriggerRequest>,
    ) -> Result<Response<InstallTriggerResponse>, Status> {
        info!("Got a request: {request:?}");

        tokio::time::sleep(Duration::from_millis(800)).await;
        let reply = InstallTriggerResponse {};

        Ok(Response::new(reply))
    }

    async fn update_trigger(
        &self,
        _request: Request<UpdateTriggerRequest>,
    ) -> Result<Response<UpdateTriggerResponse>, Status> {
        todo!()
    }

    async fn execute_trigger(
        &self,
        _request: Request<ExecuteTriggerRequest>,
    ) -> Result<Response<ExecuteTriggerResponse>, Status> {
        todo!()
    }

    async fn get_trigger(
        &self,
        _request: Request<GetTriggerRequest>,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        todo!()
    }

    async fn find_triggers(
        &self,
        _request: Request<FindTriggersRequest>,
    ) -> Result<Response<FindTriggersResponse>, Status> {
        todo!()
    }
}
