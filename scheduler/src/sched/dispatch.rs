use std::sync::Arc;

use lib::grpc_client_provider::DispatcherClientProvider;
use lib::types::{Run, Trigger};
use tracing::info;

use super::event_dispatcher::{DispatchError, DispatchMode};
use crate::sched::event_dispatcher::DispatchJob;

#[tracing::instrument(skip_all, fields(trigger_id = %trigger.id))]
pub(crate) async fn dispatch(
    trigger: Trigger,
    dispatcher_provider: Arc<DispatcherClientProvider>,
    mode: DispatchMode,
) -> Result<Run, DispatchError> {
    let mut job = DispatchJob::from_trigger(trigger, dispatcher_provider, mode);
    info!(trigger = job.trigger_id(), "async-dispatch");
    job.run().await
}
