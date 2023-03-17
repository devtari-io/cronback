use std::sync::Arc;

use proto::dispatcher_proto::DispatchRequest;
use shared::{
    grpc_client_provider::DispatcherClientProvider,
    types::{Invocation, Trigger},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum DispatchError {
    #[error("Failed while attempting to communicate with dispatcher")]
    TransportError(#[from] tonic::transport::Error),
    #[error("Dispatcher returned an error, this is unexpected!")]
    LogicalError(#[from] tonic::Status),
}

pub(crate) struct DispatchJob {
    dispatch_request: DispatchRequest,
    dispatcher_client_provider: Arc<DispatcherClientProvider>,
}

impl DispatchJob {
    pub fn from_trigger(
        trigger: Trigger,
        dispatcher_client_provider: Arc<DispatcherClientProvider>,
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
