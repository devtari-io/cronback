use std::{sync::Arc, time::Instant};

use shared::{
    grpc_client_provider::DispatcherClientProvider,
    types::{Invocation, Trigger},
};
use tracing::info;

use crate::sched::event_dispatcher::DispatchJob;

use super::event_dispatcher::DispatchError;

#[tracing::instrument(skip_all, fields(trigger_id = %trigger.id))]
pub(crate) async fn dispatch(
    trigger: Trigger,
    dispatcher_provider: Arc<DispatcherClientProvider>,
) -> Result<Invocation, DispatchError> {
    let mut job = DispatchJob::from_trigger(trigger, dispatcher_provider);

    let dispatch_instant = Instant::now();
    info!(
        trigger = job.trigger_id(),
        delay = ?Instant::now().duration_since(dispatch_instant),
        "async-dispatch",
    );
    job.run().await
}
