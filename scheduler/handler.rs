use std::{sync::Arc, time::Duration};

use tonic::{Request, Response, Status};
use tracing::info;

use crate::sched::event_scheduler::EventScheduler;
use proto::scheduler_proto::{
    scheduler_server::Scheduler, ExecuteTriggerRequest, ExecuteTriggerResponse,
    FindTriggersRequest, FindTriggersResponse, GetTriggerRequest,
    GetTriggerResponse, InstallTriggerRequest, InstallTriggerResponse,
    UpdateTriggerRequest, UpdateTriggerResponse,
};
use proto::trigger_proto::Trigger;
use shared::service::ServiceContext;

pub(crate) struct SchedulerAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
    scheduler: Arc<EventScheduler>,
}
impl SchedulerAPIHandler {
    pub(crate) fn new(
        context: ServiceContext,
        scheduler: Arc<EventScheduler>,
    ) -> Self {
        Self { context, scheduler }
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
        let (_metadata, _ext, _request) = request.into_parts();
        self.scheduler.install_trigger(Trigger::default());

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
