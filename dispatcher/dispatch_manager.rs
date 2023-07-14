use std::sync::Arc;

use dispatcher_proto::DispatchMode;
use lib::database::attempt_log_store::AttemptLogStore;
use lib::database::invocation_store::{InvocationStore, InvocationStoreError};
use lib::types::{Action, Invocation, InvocationStatus};
use metrics::{decrement_gauge, increment_gauge};
use proto::dispatcher_proto;
use thiserror::Error;
use tracing::error;

use crate::actions;

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
            | DispatchMode::Unknown => {
                panic!("Unknown dispatch mode");
            }
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

        assert_eq!(self.invocation.status, InvocationStatus::Attempting);

        let result = match &self.invocation.action {
            | Action::Webhook(web) => {
                let e = actions::webhook::WebhookActionJob {
                    webhook: web.clone(),
                    payload: self.invocation.payload.clone(),
                    invocation_id: self.invocation.id.clone(),
                    trigger_id: self.invocation.trigger.clone(),
                    project: self.invocation.project.clone(),
                    attempt_store: Arc::clone(&self.attempt_store),
                };
                e.run().await
            }
            | Action::Event(_) => unimplemented!(),
        };
        self.invocation.status = result;
        if let Err(e) = self
            .invocation_store
            .update_invocation(&self.invocation)
            .await
        {
            error!(
                "Failed to persist invocation status for invocation {} for \
                 action : {}",
                self.invocation.id, e
            );
        }

        decrement_gauge!("dispatcher.inflight_invocations_total", 1.0);

        self.invocation
    }
}
