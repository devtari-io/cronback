use std::sync::Arc;

use lib::model::ModelId;
use lib::service::ServiceContext;
use lib::types::ProjectId;
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
        let (_metadata, _ext, request) = request.into_parts();
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;
        // Creating a new trigger from install_trigger
        let reply = self.scheduler.install_trigger(project, request).await?;
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
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;
        let invocation = self
            .scheduler
            .invoke_trigger(
                project,
                request.id.clone().into(),
                request.mode().into(),
            )
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
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;

        // trigger must have an id set
        if request.id.is_empty() {
            return Err(Status::invalid_argument("Id must be set"));
        }
        let trigger = self
            .scheduler
            .get_trigger(project, request.id.into())
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
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;
        let trigger = self
            .scheduler
            .pause_trigger(project, request.id.into())
            .await?;
        Ok(Response::new(PauseTriggerResponse {
            trigger: Some(trigger.into()),
        }))
    }

    async fn resume_trigger(
        &self,
        request: Request<ResumeTriggerRequest>,
    ) -> Result<Response<ResumeTriggerResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;
        let trigger = self
            .scheduler
            .resume_trigger(project, request.id.into())
            .await?;
        Ok(Response::new(ResumeTriggerResponse {
            trigger: Some(trigger.into()),
        }))
    }

    async fn cancel_trigger(
        &self,
        request: Request<CancelTriggerRequest>,
    ) -> Result<Response<CancelTriggerResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;
        let trigger = self
            .scheduler
            .cancel_trigger(project, request.id.into())
            .await?;
        Ok(Response::new(CancelTriggerResponse {
            trigger: Some(trigger.into()),
        }))
    }

    async fn list_triggers(
        &self,
        request: Request<ListTriggersRequest>,
    ) -> Result<Response<ListTriggersResponse>, Status> {
        let (_metadata, _ext, request) = request.into_parts();
        // TODO: Move project to request metadata instead of inlining in
        // requests.
        let project =
            ProjectId::from(request.project_id.clone()).validated()?;
        let manifests = self
            .scheduler
            .list_triggers(
                project,
                request.filter.and_then(|f| f.reference),
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
