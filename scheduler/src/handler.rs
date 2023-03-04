use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::info;

use crate::sched::event_scheduler::EventScheduler;
use proto::scheduler_proto::{
    scheduler_server::Scheduler, ExecuteTriggerRequest, ExecuteTriggerResponse,
    FindTriggersRequest, FindTriggersResponse, GetTriggerRequest,
    GetTriggerResponse, InstallTriggerRequest, InstallTriggerResponse,
};
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

        let (_metadata, _ext, request) = request.into_parts();
        // basic validation for sanity
        let trigger = request
            .trigger
            .ok_or(Status::invalid_argument("Trigger must be set"))?;
        // trigger must have an id set
        if trigger.id.is_empty() {
            return Err(Status::invalid_argument("Trigger id must be set"));
        }
        if trigger.schedule.is_none() {
            return Err(Status::invalid_argument(
                "Trigger schedule must be set",
            ));
        }

        info!("Installing trigger {:?}", trigger);
        self.scheduler.install_trigger(trigger).await?;
        let reply = InstallTriggerResponse {};
        Ok(Response::new(reply))
    }

    async fn execute_trigger(
        &self,
        _request: Request<ExecuteTriggerRequest>,
    ) -> Result<Response<ExecuteTriggerResponse>, Status> {
        todo!()
    }

    async fn get_trigger(
        &self,
        request: Request<GetTriggerRequest>,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        // trigger must have an id set
        if request.id.is_empty() {
            return Err(Status::invalid_argument("Id must be set"));
        }
        let trigger = self.scheduler.get_trigger(request.id).await?;
        let reply = GetTriggerResponse {
            trigger: Some(trigger),
        };
        Ok(Response::new(reply))
    }

    async fn find_triggers(
        &self,
        _request: Request<FindTriggersRequest>,
    ) -> Result<Response<FindTriggersResponse>, Status> {
        todo!()
    }
}
