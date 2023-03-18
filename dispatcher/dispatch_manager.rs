use std::sync::Arc;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use metrics::{decrement_gauge, increment_gauge};
use proto::dispatcher_proto;
use shared::types::{Invocation, InvocationStatus, WebhookDeliveryStatus};
use thiserror::Error;

use dispatcher_proto::DispatchMode;
use tracing::{error, Instrument};

use crate::attempt_log_store::AttemptLogStore;
use crate::emits;
use crate::invocation_store::{InvocationStore, InvocationStoreError};

#[derive(Error, Debug)]
pub enum DispatcherManagerError {
    #[error("store error: {0}")]
    StoreError(#[from] InvocationStoreError),
}

pub struct DispatchManager {
    attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    invocation_store: Arc<dyn InvocationStore + Send + Sync>,
}

impl DispatchManager {
    pub fn new(
        invocation_store: Arc<dyn InvocationStore + Send + Sync>,
        attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    ) -> Self {
        // TODO: Load all non completed invocations from the database

        Self {
            invocation_store,
            attempt_store,
        }
    }

    pub async fn invoke(
        &self,
        invocation: Invocation,
        mode: DispatchMode,
    ) -> Result<Invocation, DispatcherManagerError> {
        self.invocation_store.store_invocation(&invocation).await?;

        let invocation_job = InvocationJob::from(
            invocation.clone(),
            Arc::clone(&self.invocation_store),
            Arc::clone(&self.attempt_store),
        )
        .run();

        Ok(match mode {
            | DispatchMode::Async => {
                tokio::spawn(invocation_job);
                invocation
            }
            | DispatchMode::Sync => invocation_job.await,
        })
    }
}

struct InvocationJob {
    invocation: Invocation,
    invocation_store: Arc<dyn InvocationStore + Send + Sync>,
    attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
}

impl InvocationJob {
    fn from(
        invocation: Invocation,
        invocation_store: Arc<dyn InvocationStore + Send + Sync>,
        attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    ) -> Self {
        Self {
            invocation,
            invocation_store,
            attempt_store,
        }
    }
    #[tracing::instrument(skip(self))]
    async fn run(mut self) -> Invocation {
        increment_gauge!("dispatcher.inflight_invocations_total", 1.0);
        let mut emits = FuturesUnordered::new();
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
                    let attempt_store = Arc::clone(&self.attempt_store);
                    emits.push(
                        async move {
                            let e = emits::webhook::WebhookEmitJob {
                                webhook: web.webhook.clone(),
                                payload: p,
                                invocation_id: id,
                                trigger_id: tid,
                                owner_id: oid,
                                attempt_store,
                            };
                            web.delivery_status = e.run().await;
                            (idx, InvocationStatus::WebhookStatus(web))
                        }
                        .instrument(tracing::Span::current()),
                    );
                }
            }
        }

        while let Some((idx, result)) = emits.next().await {
            *self.invocation.status.get_mut(idx).unwrap() = result;
            if let Err(e) = self
                .invocation_store
                .store_invocation(&self.invocation)
                .await
            {
                error!("Failed to persist invocation status for invocation {} for emit #{}: {}", self.invocation.id, idx, e);
            }
        }

        decrement_gauge!("dispatcher.inflight_invocations_total", 1.0);

        self.invocation
    }
}
