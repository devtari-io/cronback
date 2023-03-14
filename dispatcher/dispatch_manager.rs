use metrics::{decrement_gauge, increment_gauge};
use shared::service::ServiceContext;
use shared::types::{Invocation, InvocationStatus, WebhookDeliveryStatus};
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};
use tracing::info;

use crate::emits;

pub struct DispatchManager {
    join_handle: JoinHandle<()>,
    processing_queue: mpsc::UnboundedSender<Invocation>,
}

impl DispatchManager {
    pub fn create_and_start(context: ServiceContext) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let handle = tokio::spawn(Self::dispatcher_loop(rx, context));

        // TODO: Load all non completed invocations from the database

        Self {
            join_handle: handle,
            processing_queue: tx,
        }
    }

    async fn dispatcher_loop(
        mut queue: mpsc::UnboundedReceiver<Invocation>,
        mut context: ServiceContext,
    ) {
        let mut join_set = JoinSet::new();
        loop {
            tokio::select! {
                invocation = queue.recv() => {
                    match invocation {
                        Some(inv) => {
                            increment_gauge!("dispatcher.inflight_invocations_total", 1.0);
                            join_set.spawn(InvocationJob::from(inv).run())
                        },
                        None => break,
                    }
                }
                _ = context.recv_shutdown_signal() => {
                    break;
                }

                _ = join_set.join_next() => {
                    decrement_gauge!("dispatcher.inflight_invocations_total", 1.0);
                    continue;
                }
            };
        }
    }

    pub fn register_invocation(
        &self,
        invocation: Invocation,
    ) -> anyhow::Result<()> {
        // Persist in the database, then enqueue the invocation to be executed
        // TODO: database persistance
        self.processing_queue.send(invocation)?;
        Ok(())
    }

    pub async fn shutdown(self) {
        self.join_handle.await.unwrap();
    }
}

struct InvocationJob {
    invocation: Invocation,
}

impl InvocationJob {
    fn from(invocation: Invocation) -> Self {
        Self { invocation }
    }
    async fn run(mut self) {
        info!(
            "Executing invocation {} for trigger {}",
            self.invocation.id, self.invocation.trigger_id
        );
        let mut join_set = JoinSet::new();
        for (idx, emit) in self.invocation.status.iter().enumerate() {
            match emit.clone() {
                | InvocationStatus::WebhookStatus(mut web) => {
                    if web.delivery_status != WebhookDeliveryStatus::Attempting
                    {
                        continue;
                    }
                    let p = self.invocation.payload.clone();
                    let id = self.invocation.id.clone();
                    let tid = self.invocation.trigger_id.clone();
                    let oid = self.invocation.owner_id.clone();

                    join_set.spawn(async move {
                        let e = emits::webhook::WebhookEmitJob {
                            webhook: web.webhook.clone(),
                            payload: p,
                            invocation_id: id,
                            trigger_id: tid,
                            owner_id: oid,
                        };
                        web.delivery_status = e.run().await;
                        (idx, InvocationStatus::WebhookStatus(web))
                    });
                }
            }
        }

        while let Some(Ok((idx, result))) = join_set.join_next().await {
            *self.invocation.status.get_mut(idx).unwrap() = result;

            // TODO: Update the database
        }
    }
}
