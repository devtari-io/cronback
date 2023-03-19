use std::{sync::Arc, time::Instant};

use shared::{
    grpc_client_provider::DispatcherClientProvider,
    types::{Invocation, Trigger},
};
use tracing::info;

use super::event_dispatcher::DispatchError;
use super::event_dispatcher::DispatchMode;
use crate::sched::event_dispatcher::DispatchJob;

#[tracing::instrument(skip_all, fields(trigger_id = %trigger.id))]
pub(crate) async fn dispatch(
    trigger: Trigger,
    dispatcher_provider: Arc<DispatcherClientProvider>,
    mode: DispatchMode,
) -> Result<Invocation, DispatchError> {
    let mut job = DispatchJob::from_trigger(trigger, dispatcher_provider, mode);

    let dispatch_instant = Instant::now();
    info!(
        trigger = job.trigger_id(),
        delay = ?Instant::now().duration_since(dispatch_instant),
        "async-dispatch",
    );
    job.run().await
}
