use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::info;

use crate::sched::event_scheduler::EventScheduler;
use proto::scheduler_proto::{
    scheduler_server::Scheduler, ExecuteTriggerRequest, ExecuteTriggerResponse,
    FindTriggersRequest, FindTriggersResponse, GetTriggerRequest,
    GetTriggerResponse, InstallTriggerRequest, InstallTriggerResponse,
    UpdateTriggerRequest, UpdateTriggerResponse,
};
use proto::trigger_proto::Cron;
use proto::trigger_proto::{self, Schedule};
use proto::trigger_proto::{Trigger, TriggerStatus};
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

        let reply = InstallTriggerResponse {};
        let (_metadata, _ext, _request) = request.into_parts();
        let id = format!("trig_{}", rand::random::<u64>());
        let sec = format!("{}", rand::random::<u16>() % 59);
        let trigger = Trigger {
            id,
            owner_id: "asoli".to_owned(),
            reference_id: None,
            name: None,
            description: None,
            created_at: None,
            endpoint: None,
            payload: None,
            timeout: None,
            status: TriggerStatus::Active.into(),
            event_retry_policy: None,
            on_success: None,
            on_failure: None,
            schedule: Some(Schedule {
                schedule: Some(trigger_proto::schedule::Schedule::Cron(Cron {
                    cron: format!("{} * * * * *", sec),
                    timezone: "Europe/London".into(),
                    events_limit: 4,
                })),
            }),
        };
        info!("Installing trigger {:?}", trigger);
        self.scheduler
            .install_trigger(trigger)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

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
