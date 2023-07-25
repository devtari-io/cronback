use std::sync::Arc;

use dto::{FromProto, IntoProto};
use lib::clients::dispatcher_client::ScopedDispatcherSvcClient;
use lib::grpc_client_provider::{
    GrpcClientError,
    GrpcClientFactory,
    GrpcClientProvider,
};
use lib::prelude::{RequestContext, *};
use proto::dispatcher_svc::{self, DispatchRequest};
use proto::runs::Run;
use thiserror::Error;
use tracing::info;

use crate::scheduler::db_model::Trigger;

#[tracing::instrument(skip_all, fields(trigger_id = %trigger.id))]
pub(crate) async fn dispatch(
    context: RequestContext,
    trigger: Trigger,
    dispatch_clients: Arc<GrpcClientProvider<ScopedDispatcherSvcClient>>,
    mode: DispatchMode,
) -> Result<Run, DispatchError> {
    let mut job =
        DispatchJob::from_trigger(context, trigger, dispatch_clients, mode);
    info!(trigger = job.trigger_id(), "async-dispatch");
    job.run().await
}

#[derive(Error, Debug)]
pub enum DispatchError {
    #[error("Failed while attempting to communicate with dispatcher")]
    Transport(#[from] tonic::transport::Error),
    #[error("Dispatcher returned an error, this is unexpected!")]
    Logical(#[from] tonic::Status),
    #[error("Failed while attempting to connect to dispatcher")]
    GrpcClient(#[from] GrpcClientError),
}

#[derive(Debug, FromProto, IntoProto)]
#[proto(target = "proto::dispatcher_svc::DispatchMode")]
pub enum DispatchMode {
    Sync,
    Async,
}

pub struct DispatchJob {
    context: RequestContext,
    dispatch_request: DispatchRequest,
    dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherSvcClient>>,
}

impl DispatchJob {
    pub fn from_trigger(
        context: RequestContext,
        trigger: Trigger,
        dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherSvcClient>>,
        mode: DispatchMode,
    ) -> Self {
        Self {
            context,
            dispatch_request: DispatchRequest {
                trigger_id: Some(trigger.id.into()),
                action: Some(trigger.action.into()),
                payload: trigger.payload.map(|p| p.into()),
                mode: dispatcher_svc::DispatchMode::from(mode).into(),
            },
            dispatcher_clients,
        }
    }

    pub fn trigger_id(&self) -> &str {
        &self.dispatch_request.trigger_id.unwrap_ref().value
    }

    pub async fn run(&mut self) -> Result<Run, DispatchError> {
        let mut client = self
            .dispatcher_clients
            .get_client(&self.context.request_id, &self.context.project_id)
            .await?;

        let resp = client.dispatch(self.dispatch_request.clone()).await?;
        let run = resp.into_inner().run.unwrap();
        Ok(run)
    }
}
