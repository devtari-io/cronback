use std::sync::Arc;

use lib::prelude::*;
use lib::service::ServiceContext;
use proto::scheduler_svc::scheduler_svc_server::SchedulerSvc;
use proto::scheduler_svc::{
    CancelTriggerRequest,
    CancelTriggerResponse,
    DeleteProjectTriggersRequest,
    DeleteProjectTriggersResponse,
    DeleteTriggerRequest,
    DeleteTriggerResponse,
    GetTriggerIdRequest,
    GetTriggerIdResponse,
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

use super::db_model::triggers;
use super::spinner::controller::SpinnerController;
use super::SchedulerService;

pub(crate) struct SchedulerSvcHandler {
    #[allow(unused)]
    context: ServiceContext<SchedulerService>,
    scheduler: Arc<SpinnerController>,
}
impl SchedulerSvcHandler {
    pub(crate) fn new(
        context: ServiceContext<SchedulerService>,
        scheduler: Arc<SpinnerController>,
    ) -> Self {
        Self { context, scheduler }
    }
}

#[tonic::async_trait]
impl SchedulerSvc for SchedulerSvcHandler {
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
        // A trigger that exists can run regardless of its state.
        let request = request.into_inner();
        let mode = request.mode.into();
        let run = self.scheduler.run_trigger(ctx, request.name, mode).await?;
        Ok(Response::new(RunTriggerResponse { run: Some(run) }))
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

    async fn delete_trigger(
        &self,
        request: Request<DeleteTriggerRequest>,
    ) -> Result<Response<DeleteTriggerResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();
        self.scheduler.delete_trigger(ctx, request.name).await?;
        Ok(Response::new(DeleteTriggerResponse {}))
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

    async fn delete_project_triggers(
        &self,
        request: Request<DeleteProjectTriggersRequest>,
    ) -> Result<Response<DeleteProjectTriggersResponse>, Status> {
        let ctx = request.context()?;
        self.scheduler.delete_project_triggers(ctx).await?;
        Ok(Response::new(DeleteProjectTriggersResponse {}))
    }

    async fn get_trigger_id(
        &self,
        request: Request<GetTriggerIdRequest>,
    ) -> Result<Response<GetTriggerIdResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();

        let trigger_id = self
            .scheduler
            .get_trigger_id(&ctx.project_id, &request.name)
            .await?;
        Ok(Response::new(GetTriggerIdResponse {
            id: Some(trigger_id.into()),
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
