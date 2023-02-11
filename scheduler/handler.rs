use std::{sync::Arc, time::Duration};

use proto::scheduler_proto::{
    scheduler_server::Scheduler, ExecuteTriggerRequest, ExecuteTriggerResponse,
    FindTriggersRequest, FindTriggersResponse, GetTriggerRequest, GetTriggerResponse,
    InstallTriggerRequest, InstallTriggerResponse, UpdateTriggerRequest, UpdateTriggerResponse,
};
use tonic::{Request, Response, Status};

use shared::config::ConfigLoader;
use tracing::info;

pub(crate) struct SchedulerAPIHandler {
    pub config_loader: Arc<ConfigLoader>,
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
        request: Request<UpdateTriggerRequest>,
    ) -> Result<Response<UpdateTriggerResponse>, Status> {
        todo!()
    }

    async fn execute_trigger(
        &self,
        request: Request<ExecuteTriggerRequest>,
    ) -> Result<Response<ExecuteTriggerResponse>, Status> {
        todo!()
    }

    async fn get_trigger(
        &self,
        request: Request<GetTriggerRequest>,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        todo!()
    }

    async fn find_triggers(
        &self,
        request: Request<FindTriggersRequest>,
    ) -> Result<Response<FindTriggersResponse>, Status> {
        todo!()
    }
}
