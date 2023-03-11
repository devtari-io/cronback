use std::sync::Arc;

use chrono::Utc;
use chrono_tz::UTC;
use tonic::{Request, Response, Status};
use tracing::info;

use crate::sched::event_scheduler::EventScheduler;
use proto::scheduler_proto::{
    scheduler_server::Scheduler, ExecuteTriggerRequest, ExecuteTriggerResponse,
    FindTriggersRequest, FindTriggersResponse, GetTriggerRequest,
    GetTriggerResponse, InstallTrigger, InstallTriggerRequest,
    InstallTriggerResponse,
};
use shared::{
    service::ServiceContext,
    types::{OwnerId, Trigger, TriggerId},
};

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
        let install_trigger: InstallTrigger =
            request.install_trigger.ok_or_else(|| {
                Status::invalid_argument("Missing install_trigger in request!")
            })?;

        info!("Installing trigger {:?}", install_trigger);
        // TODO: Instantiate a trigger, we will lookup the database to check for reference id

        // Creating a new trigger from install_trigger
        let trigger = self.scheduler.install_trigger(install_trigger).await?;
        let reply = InstallTriggerResponse {
            trigger: Some(trigger.into()),
        };
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
        let trigger = self.scheduler.get_trigger(request.id.into()).await?;
        let reply = GetTriggerResponse {
            trigger: Some(trigger.into()),
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
