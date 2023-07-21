use std::sync::Arc;

use chrono::Utc;
use futures::TryFutureExt;
use lib::database::run_store::RunStore;
use lib::database::DatabaseError;
use lib::e;
use lib::prelude::TonicRequestExt;
use lib::service::ServiceContext;
use lib::types::{Run, RunId, RunStatus, TriggerId};
use metrics::counter;
use proto::common::PaginationIn;
use proto::dispatcher_proto::dispatcher_server::Dispatcher;
use proto::dispatcher_proto::{
    DispatchRequest,
    DispatchResponse,
    GetRunRequest,
    GetRunResponse,
    ListRunsRequest,
    ListRunsResponse,
};
use thiserror::Error;
use tonic::{Request, Response, Status};

use super::dispatch_manager::DispatchManager;

pub(crate) struct DispatcherAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
    dispatch_manager: DispatchManager,
    run_store: Arc<dyn RunStore + Send + Sync>,
}

impl DispatcherAPIHandler {
    pub fn new(
        context: ServiceContext,
        dispatch_manager: DispatchManager,
        run_store: Arc<dyn RunStore + Send + Sync>,
    ) -> Self {
        Self {
            context,
            dispatch_manager,
            run_store,
        }
    }
}

#[tonic::async_trait]
impl Dispatcher for DispatcherAPIHandler {
    async fn dispatch(
        &self,
        request: Request<DispatchRequest>,
    ) -> Result<Response<DispatchResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();

        let dispatch_mode = request.mode();
        let run_id = RunId::generate(&ctx.project_id);

        let run = Run {
            id: run_id.into(),
            trigger_id: request.trigger_id.unwrap().into(),
            project_id: ctx.project_id.clone(),
            created_at: Utc::now(),
            payload: request.payload.map(|p| p.into()),
            action: request.action.unwrap().into(),
            status: RunStatus::Attempting,
            latest_attempt_id: None,
            latest_attempt: None,
        };

        counter!("dispatcher.runs_total", 1);
        e!(
            context = ctx,
            RunCreated {
                meta: run.meta().into()
            }
        );
        let run = self
            .dispatch_manager
            .run(run, dispatch_mode)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DispatchResponse {
            run: Some(run.into()),
        }))
    }

    async fn get_run(
        &self,
        request: Request<GetRunRequest>,
    ) -> Result<Response<GetRunResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();

        let run_id: RunId = request.run_id.unwrap().into();

        let run = self
            .run_store
            .get_run(&ctx.project_id, &run_id)
            .map_err(DispatcherHandlerError::Store)
            .await?;

        match run {
            | Some(run) => {
                Ok(Response::new(GetRunResponse {
                    run: Some(run.into()),
                }))
            }
            | None => {
                Err(DispatcherHandlerError::NotFound(run_id.to_string()).into())
            }
        }
    }

    async fn list_runs(
        &self,
        request: Request<ListRunsRequest>,
    ) -> Result<Response<ListRunsResponse>, Status> {
        let ctx = request.context()?;
        let request = request.into_inner();
        let trigger_id: TriggerId = request.trigger_id.unwrap().into();
        let pagination: PaginationIn = request.pagination.unwrap();

        let runs = self
            .run_store
            .get_runs_by_trigger(&ctx.project_id, &trigger_id, pagination)
            .await
            .map_err(DispatcherHandlerError::Store)?;

        Ok(Response::new(ListRunsResponse {
            runs: runs.data.into_iter().map(Into::into).collect(),
            pagination: Some(runs.pagination),
        }))
    }
}

#[derive(Error, Debug)]
pub(crate) enum DispatcherHandlerError {
    #[error("Run '{0}' is unknown to this dispatcher!")]
    NotFound(String),
    #[error("Operation on underlying database failed: {0}")]
    Store(#[from] DatabaseError),
}

impl From<DispatcherHandlerError> for Status {
    fn from(e: DispatcherHandlerError) -> Self {
        // match variants of TriggerError
        match e {
            | DispatcherHandlerError::NotFound(e) => Status::not_found(e),
            | e => Status::invalid_argument(e.to_string()),
        }
    }
}
