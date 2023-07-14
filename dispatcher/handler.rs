use chrono::Utc;
use lib::model::ModelId;
use lib::service::ServiceContext;
use lib::types::{ProjectId, Run, RunId, RunStatus};
use metrics::counter;
use proto::dispatcher_proto::dispatcher_server::Dispatcher;
use proto::dispatcher_proto::{DispatchRequest, DispatchResponse};
use tonic::{Request, Response, Status};

use crate::dispatch_manager::DispatchManager;

pub(crate) struct DispatcherAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
    dispatch_manager: DispatchManager,
}

impl DispatcherAPIHandler {
    pub fn new(
        context: ServiceContext,
        dispatch_manager: DispatchManager,
    ) -> Self {
        Self {
            context,
            dispatch_manager,
        }
    }
}

#[tonic::async_trait]
impl Dispatcher for DispatcherAPIHandler {
    async fn dispatch(
        &self,
        request: Request<DispatchRequest>,
    ) -> Result<Response<DispatchResponse>, Status> {
        let (_metadata, _extensions, request) = request.into_parts();

        let dispatch_mode = request.mode();
        let project_id = ProjectId::from(request.project_id).validated()?;
        let run_id = RunId::generate(&project_id);

        let run = Run {
            id: run_id.into(),
            trigger_id: request.trigger_id.into(),
            project_id,
            created_at: Utc::now(),
            payload: request.payload.map(|p| p.into()),
            action: request.action.unwrap().into(),
            status: RunStatus::Attempting,
        };

        counter!("dispatcher.runs_total", 1);
        let run = self
            .dispatch_manager
            .run(run, dispatch_mode)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DispatchResponse {
            run: Some(run.into()),
        }))
    }
}
