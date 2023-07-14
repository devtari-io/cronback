use std::sync::Arc;

use lib::grpc_client_provider::DispatcherClientProvider;
use lib::types::{Run, Trigger};
use proto::dispatcher_proto::{self, DispatchRequest};
use proto::scheduler_proto;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum DispatchError {
    #[error("Failed while attempting to communicate with dispatcher")]
    TransportError(#[from] tonic::transport::Error),
    #[error("Dispatcher returned an error, this is unexpected!")]
    LogicalError(#[from] tonic::Status),
}

pub(crate) enum DispatchMode {
    Sync,
    Async,
}

impl From<DispatchMode> for dispatcher_proto::DispatchMode {
    fn from(value: DispatchMode) -> Self {
        match value {
            | DispatchMode::Sync => dispatcher_proto::DispatchMode::Sync,
            | DispatchMode::Async => dispatcher_proto::DispatchMode::Async,
        }
    }
}

impl From<scheduler_proto::RunMode> for DispatchMode {
    fn from(value: scheduler_proto::RunMode) -> Self {
        match value {
            | scheduler_proto::RunMode::Unknown => {
                panic!("Unknown run mode");
            }
            | scheduler_proto::RunMode::Sync => DispatchMode::Sync,
            | scheduler_proto::RunMode::Async => DispatchMode::Async,
        }
    }
}

pub(crate) struct DispatchJob {
    dispatch_request: DispatchRequest,
    dispatcher_client_provider: Arc<DispatcherClientProvider>,
}

impl DispatchJob {
    pub fn from_trigger(
        trigger: Trigger,
        dispatcher_client_provider: Arc<DispatcherClientProvider>,
        mode: DispatchMode,
    ) -> Self {
        Self {
            dispatch_request: DispatchRequest {
                trigger_id: trigger.id.to_string(),
                project_id: trigger.project.to_string(),
                action: Some(trigger.action.into()),
                payload: trigger.payload.map(|p| p.into()),
                mode: dispatcher_proto::DispatchMode::from(mode).into(),
            },
            dispatcher_client_provider,
        }
    }

    pub fn trigger_id(&self) -> &str {
        &self.dispatch_request.trigger_id
    }

    pub async fn run(&mut self) -> Result<Run, DispatchError> {
        let mut client =
            self.dispatcher_client_provider.get_or_create().await?;

        let resp = client.dispatch(self.dispatch_request.clone()).await?;
        let run = resp.into_inner().run.unwrap();
        Ok(run.into())
    }
}
