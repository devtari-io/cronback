use std::sync::Arc;

use chrono::Utc;
use futures::TryFutureExt;
use lib::database::attempt_log_store::AttemptLogStore;
use lib::database::run_store::RunStore;
use lib::database::DatabaseError;
use lib::prelude::TonicRequestExt;
use lib::service::ServiceContext;
use lib::types::{ProjectId, Run, RunId, RunStatus, TriggerId};
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

use crate::dispatch_manager::DispatchManager;

const NUM_INLINE_ATTEMPTS_PER_RUN: i32 = 5;

pub(crate) struct DispatcherAPIHandler {
    #[allow(unused)]
    context: ServiceContext,
    dispatch_manager: DispatchManager,
    run_store: Arc<dyn RunStore + Send + Sync>,
    attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
}

impl DispatcherAPIHandler {
    pub fn new(
        context: ServiceContext,
        dispatch_manager: DispatchManager,
        run_store: Arc<dyn RunStore + Send + Sync>,
        attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    ) -> Self {
        Self {
            context,
            dispatch_manager,
            run_store,
            attempt_store,
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
        let project_id: lib::prelude::ValidShardedId<ProjectId> =
            request.project_id.unwrap().into();
        let run_id = RunId::generate(&project_id);

        let run = Run {
            id: run_id.into(),
            trigger_id: request.trigger_id.unwrap().into(),
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

    async fn get_run(
        &self,
        request: Request<GetRunRequest>,
    ) -> Result<Response<GetRunResponse>, Status> {
        let project_id = request.context()?.project_id;
        let (_metadata, _extensions, request) = request.into_parts();

        let run_id: RunId = request.run_id.unwrap().into();

        let (run, latest_attempt) = tokio::join!(
            self.run_store
                .get_run(&project_id, &run_id)
                .map_err(DispatcherHandlerError::Store),
            self.attempt_store
                .get_attempts_for_run(
                    &project_id,
                    &run_id,
                    PaginationIn {
                        limit: NUM_INLINE_ATTEMPTS_PER_RUN,
                        cursor: None
                    },
                )
                .map_err(DispatcherHandlerError::Store),
        );

        match run? {
            | Some(run) => {
                Ok(Response::new(GetRunResponse {
                    run: Some(run.into()),
                    latest_attempts: latest_attempt?
                        .data
                        .into_iter()
                        .map(Into::into)
                        .collect(),
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
        let project_id = request.context()?.project_id;
        let (_metadata, _extensions, request) = request.into_parts();

        let trigger_id: TriggerId = request.trigger_id.unwrap().into();
        let pagination: PaginationIn = request.pagination.unwrap();

        let runs = self
            .run_store
            .get_runs_by_trigger(&project_id, &trigger_id, pagination)
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
