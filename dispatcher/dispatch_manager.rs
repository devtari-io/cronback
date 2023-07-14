use std::sync::Arc;

use dispatcher_proto::DispatchMode;
use lib::database::attempt_log_store::AttemptLogStore;
use lib::database::run_store::{RunStore, RunStoreError};
use lib::types::{Action, Run, RunStatus};
use metrics::{decrement_gauge, increment_gauge};
use proto::dispatcher_proto;
use thiserror::Error;
use tracing::error;

use crate::actions;

#[derive(Error, Debug)]
pub enum DispatcherManagerError {
    #[error("store error: {0}")]
    StoreError(#[from] RunStoreError),
}

pub struct DispatchManager {
    attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    run_store: Arc<dyn RunStore + Send + Sync>,
}

impl DispatchManager {
    pub fn new(
        run_store: Arc<dyn RunStore + Send + Sync>,
        attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    ) -> Self {
        // TODO: Load all non completed runs from the database

        Self {
            run_store,
            attempt_store,
        }
    }

    pub async fn run(
        &self,
        run: Run,
        mode: DispatchMode,
    ) -> Result<Run, DispatcherManagerError> {
        self.run_store.store_run(run.clone()).await?;

        let run_job = RunJob::from(
            run.clone(),
            Arc::clone(&self.run_store),
            Arc::clone(&self.attempt_store),
        )
        .run();

        Ok(match mode {
            | DispatchMode::Unknown => {
                panic!("Unknown dispatch mode");
            }
            | DispatchMode::Async => {
                tokio::spawn(run_job);
                run
            }
            | DispatchMode::Sync => run_job.await,
        })
    }
}

struct RunJob {
    run: Run,
    run_store: Arc<dyn RunStore + Send + Sync>,
    attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
}

impl RunJob {
    fn from(
        run: Run,
        run_store: Arc<dyn RunStore + Send + Sync>,
        attempt_store: Arc<dyn AttemptLogStore + Send + Sync>,
    ) -> Self {
        Self {
            run,
            run_store,
            attempt_store,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn run(mut self) -> Run {
        increment_gauge!("dispatcher.inflight_runs_total", 1.0);

        assert_eq!(self.run.status, RunStatus::Attempting);

        let result = match &self.run.action {
            | Action::Webhook(web) => {
                let e = actions::webhook::WebhookActionJob {
                    webhook: web.clone(),
                    payload: self.run.payload.clone(),
                    run_id: self.run.id.clone(),
                    trigger_id: self.run.trigger.clone(),
                    project: self.run.project.clone(),
                    attempt_store: Arc::clone(&self.attempt_store),
                };
                e.run().await
            }
            | Action::Event(_) => unimplemented!(),
        };
        self.run.status = result;
        if let Err(e) = self.run_store.update_run(self.run.clone()).await {
            error!(
                "Failed to persist run status for run {} for action : {}",
                self.run.id, e
            );
        }

        decrement_gauge!("dispatcher.inflight_runs_total", 1.0);

        self.run
    }
}
