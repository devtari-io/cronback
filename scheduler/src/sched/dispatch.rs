use std::sync::Arc;

use lib::clients::dispatcher_client::ScopedDispatcherClient;
use lib::grpc_client_provider::GrpcClientProvider;
use lib::prelude::RequestContext;
use lib::types::{Run, Trigger};
use tracing::info;

use super::event_dispatcher::{DispatchError, DispatchMode};
use crate::sched::event_dispatcher::DispatchJob;

#[tracing::instrument(skip_all, fields(trigger_id = %trigger.id))]
pub(crate) async fn dispatch(
    context: RequestContext,
    trigger: Trigger,
    dispatch_clients: Arc<GrpcClientProvider<ScopedDispatcherClient>>,
    mode: DispatchMode,
) -> Result<Run, DispatchError> {
    let mut job =
        DispatchJob::from_trigger(context, trigger, dispatch_clients, mode);
    info!(trigger = job.trigger_id(), "async-dispatch");
    job.run().await
}
