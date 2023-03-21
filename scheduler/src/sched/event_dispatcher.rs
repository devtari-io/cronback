use std::sync::Arc;

use proto::dispatcher_proto::{self, DispatchRequest};
use shared::grpc_client_provider::DispatcherClientProvider;
use shared::types::{Invocation, Trigger};
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
                owner_id: trigger.owner_id.to_string(),
                emits: trigger
                    .emit
                    .into_iter()
                    .map(|e| e.into())
                    .collect::<Vec<_>>(),
                payload: Some(trigger.payload.into()),
                on_success: None, // TODO
                on_failure: None, // TODO
                mode: dispatcher_proto::DispatchMode::from(mode).into(),
            },
            dispatcher_client_provider,
        }
    }

    pub fn trigger_id(&self) -> &str {
        &self.dispatch_request.trigger_id
    }

    pub async fn run(&mut self) -> Result<Invocation, DispatchError> {
        let mut client =
            self.dispatcher_client_provider.get_or_create().await?;

        let resp = client.dispatch(self.dispatch_request.clone()).await?;
        let invocation = resp.into_inner().invocation.unwrap();
        Ok(invocation.into())
    }
}
