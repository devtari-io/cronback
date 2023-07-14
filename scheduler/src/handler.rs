use std::sync::Arc;

use lib::database::models::triggers;
use lib::prelude::*;
use lib::service::ServiceContext;
use proto::scheduler_proto::scheduler_server::Scheduler;
use proto::scheduler_proto::{
    CancelTriggerRequest,
    CancelTriggerResponse,
    GetTriggerRequest,
    GetTriggerResponse,
    ListTriggersFilter,
    ListTriggersRequest,
    ListTriggersResponse,
    PauseTriggerRequest,
    PauseTriggerResponse,
    ResumeTriggerRequest,
    ResumeTriggerResponse,
    RunTriggerRequest,
    RunTriggerResponse,
    UpsertTriggerRequest,
    UpsertTriggerResponse,
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
    async fn upsert_trigger(
        &self,
        request: Request<UpsertTriggerRequest>,
    ) -> Result<Response<UpsertTriggerResponse>, Status> {
        let ctx = request.context()?;
        // Creating a new trigger from install_trigger
        let reply = self
            .scheduler
            .upsert_trigger(ctx, request.into_inner())
            .await?;
        Ok(Response::new(reply))
    }

    async fn run_trigger(
        &self,
        request: Request<RunTriggerRequest>,
    ) -> Result<Response<RunTriggerResponse>, Status> {
        let ctx = request.context()?;
        // check if trigger exists
        // A trigger that exists will run regardless of its state
        // manual run has nothing to do with the spinner or event
        // scheduler
        //
        let request = request.into_inner();
        let mode = request.mode.into();
        let run = self.scheduler.run_trigger(ctx, request.name, mode).await?;
        Ok(Response::new(RunTriggerResponse {
            run: Some(run.into()),
        }))
    }

    async fn get_trigger(
        &self,
        request: Request<GetTriggerRequest>,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();
        let trigger = self.scheduler.get_trigger(ctx, request.name).await?;
        let reply = GetTriggerResponse {
            trigger: Some(trigger.into()),
        };
        Ok(Response::new(reply))
    }

    async fn pause_trigger(
        &self,
        request: Request<PauseTriggerRequest>,
    ) -> Result<Response<PauseTriggerResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();
        let trigger = self.scheduler.pause_trigger(ctx, request.name).await?;
        Ok(Response::new(PauseTriggerResponse {
            trigger: Some(trigger.into()),
        }))
    }

    async fn resume_trigger(
        &self,
        request: Request<ResumeTriggerRequest>,
    ) -> Result<Response<ResumeTriggerResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();
        let name = request.name;
        let trigger = self.scheduler.resume_trigger(ctx, name).await?;
        Ok(Response::new(ResumeTriggerResponse {
            trigger: Some(trigger.into()),
        }))
    }

    async fn cancel_trigger(
        &self,
        request: Request<CancelTriggerRequest>,
    ) -> Result<Response<CancelTriggerResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();
        let trigger = self.scheduler.cancel_trigger(ctx, request.name).await?;
        Ok(Response::new(CancelTriggerResponse {
            trigger: Some(trigger.into()),
        }))
    }

    async fn list_triggers(
        &self,
        request: Request<ListTriggersRequest>,
    ) -> Result<Response<ListTriggersResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();

        let statuses = list_filter_into_parts(request.filter);
        let paginated_result = self
            .scheduler
            .list_triggers(
                ctx,
                statuses,
                request.pagination.unwrap_or_default(),
            )
            .await?;
        Ok(Response::new(ListTriggersResponse {
            triggers: paginated_result
                .data
                .into_iter()
                .map(Into::into)
                .collect(),
            pagination: Some(paginated_result.pagination),
        }))
    }
}

fn list_filter_into_parts(
    filter: Option<ListTriggersFilter>,
) -> Option<Vec<triggers::Status>> {
    let Some(filter) = filter else {
        return None;
    };

    let ListTriggersFilter { statuses } = filter;
    if !statuses.is_empty() {
        Some(
            statuses
                .into_iter()
                .map(Into::into)
                .collect::<Vec<triggers::Status>>(),
        )
    } else {
        None
    }
}
