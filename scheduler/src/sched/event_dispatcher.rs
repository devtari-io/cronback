use std::sync::Arc;

use dto::{FromProto, IntoProto};
use lib::clients::dispatcher_client::ScopedDispatcherClient;
use lib::grpc_client_provider::{
    GrpcClientError,
    GrpcClientFactory,
    GrpcClientProvider,
};
use lib::prelude::*;
use lib::types::{Run, Trigger};
use proto::dispatcher_proto::{self, DispatchRequest};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum DispatchError {
    #[error("Failed while attempting to communicate with dispatcher")]
    Transport(#[from] tonic::transport::Error),
    #[error("Dispatcher returned an error, this is unexpected!")]
    Logical(#[from] tonic::Status),
    #[error("Failed while attempting to connect to dispatcher")]
    GrpcClient(#[from] GrpcClientError),
}

#[derive(FromProto, IntoProto)]
#[proto(target = "proto::dispatcher_proto::DispatchMode")]
pub(crate) enum DispatchMode {
    Sync,
    Async,
}

pub(crate) struct DispatchJob {
    context: RequestContext,
    dispatch_request: DispatchRequest,
    dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherClient>>,
}

impl DispatchJob {
    pub fn from_trigger(
        context: RequestContext,
        trigger: Trigger,
        dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherClient>>,
        mode: DispatchMode,
    ) -> Self {
        Self {
            context,
            dispatch_request: DispatchRequest {
                trigger_id: trigger.id.to_string(),
                project_id: trigger.project_id.to_string(),
                action: Some(trigger.action.into()),
                payload: trigger.payload.map(|p| p.into()),
                mode: dispatcher_proto::DispatchMode::from(mode).into(),
            },
            dispatcher_clients,
        }
    }

    pub fn trigger_id(&self) -> &str {
        &self.dispatch_request.trigger_id
    }

    pub async fn run(&mut self) -> Result<Run, DispatchError> {
        let mut client = self
            .dispatcher_clients
            .get_client(&self.context.request_id, &self.context.project_id)
            .await?;

        let resp = client.dispatch(self.dispatch_request.clone()).await?;
        let run = resp.into_inner().run.unwrap();
        Ok(run.into())
    }
}
