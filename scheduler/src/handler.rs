use std::sync::Arc;

use lib::service::ServiceContext;
use proto::scheduler_proto::scheduler_server::Scheduler;
use proto::scheduler_proto::{
    CancelTriggerRequest,
    CancelTriggerResponse,
    FindTriggersRequest,
    FindTriggersResponse,
    GetTriggerRequest,
    GetTriggerResponse,
    InstallTriggerRequest,
    InstallTriggerResponse,
    InvokeTriggerRequest,
    InvokeTriggerResponse,
    ListTriggersRequest,
    ListTriggersResponse,
    PauseTriggerRequest,
    PauseTriggerResponse,
    ResumeTriggerRequest,
    ResumeTriggerResponse,
};
use tonic::{Request, Response, Status};
use tracing::info;

use crate::sched::event_scheduler::EventScheduler;

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
        info!("Installing trigger {:?}", request);
        // TODO: Instantiate a trigger, we will lookup the database to check for
        // reference id

        // Creating a new trigger from install_trigger
        let trigger = self.scheduler.install_trigger(request).await?;
        let reply = InstallTriggerResponse {
            trigger: Some(trigger.into()),
        };
        Ok(Response::new(reply))
    }

    async fn invoke_trigger(
        &self,
        request: Request<InvokeTriggerRequest>,
    ) -> Result<Response<InvokeTriggerResponse>, Status> {
        // check if trigger exists
        // A trigger that exists will be invoked regardless of its state
        // manual invocation has nothing to do with the spinner or event
        // scheduler
        //
        let (_metadata, _ext, request) = request.into_parts();
        info!(request.id, "Invoking trigger");
        let invocation = self
            .scheduler
            .invoke_trigger(request.owner_id.into(), request.id.into())
            .await?;
        Ok(Response::new(InvokeTriggerResponse {
            invocation: Some(invocation.into()),
        }))
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
        let trigger = self
            .scheduler
            .get_trigger(request.owner_id.into(), request.id.into())
            .await?;
        let reply = GetTriggerResponse {
            trigger: Some(trigger.into()),
        };
        Ok(Response::new(reply))
    }

    async fn pause_trigger(
        &self,
        request: Request<PauseTriggerRequest>,
    ) -> Result<Response<PauseTriggerResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        info!(request.id, "Pausing trigger");
        let trigger = self
            .scheduler
            .pause_trigger(request.owner_id.into(), request.id.into())
            .await?;
        Ok(Response::new(PauseTriggerResponse {
            trigger: Some(trigger.into_manifest().into()),
        }))
    }

    async fn resume_trigger(
        &self,
        request: Request<ResumeTriggerRequest>,
    ) -> Result<Response<ResumeTriggerResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        info!(request.id, "Resuming trigger");
        let trigger = self
            .scheduler
            .resume_trigger(request.owner_id.into(), request.id.into())
            .await?;
        Ok(Response::new(ResumeTriggerResponse {
            trigger: Some(trigger.into_manifest().into()),
        }))
    }

    async fn cancel_trigger(
        &self,
        request: Request<CancelTriggerRequest>,
    ) -> Result<Response<CancelTriggerResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        info!(request.id, "Canceling trigger");
        let trigger = self
            .scheduler
            .cancel_trigger(request.owner_id.into(), request.id.into())
            .await?;
        Ok(Response::new(CancelTriggerResponse {
            trigger: Some(trigger.into_manifest().into()),
        }))
    }

    async fn list_triggers(
        &self,
        request: Request<ListTriggersRequest>,
    ) -> Result<Response<ListTriggersResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        let manifests = self
            .scheduler
            .list_triggers(
                request.owner_id.into(),
                request.limit as usize,
                request.before.map(Into::into),
                request.after.map(Into::into),
            )
            .await?;
        Ok(Response::new(ListTriggersResponse {
            triggers: manifests.into_iter().map(Into::into).collect(),
        }))
    }

    async fn find_triggers(
        &self,
        _request: Request<FindTriggersRequest>,
    ) -> Result<Response<FindTriggersResponse>, Status> {
        todo!()
    }
}
