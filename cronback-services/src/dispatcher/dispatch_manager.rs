use std::debug_assert;
use std::time::Duration;

use chrono::Utc;
use dispatcher_svc::DispatchMode;
use lib::prelude::*;
use metrics::{decrement_gauge, increment_gauge};
use proto::dispatcher_svc;
use thiserror::Error;
use tracing::{error, info};

use super::attempt_store::AttemptStore;
use super::db_model::runs::RunStatus;
use super::db_model::Run;
use super::run_store::{RunStore, RunStoreError};
use super::webhook_action::WebhookActionJob;

#[derive(Error, Debug)]
pub enum DispatcherManagerError {
    #[error("store error: {0}")]
    Store(#[from] RunStoreError),
}

pub struct DispatchManager {
    _cell_id: u32,
    attempt_store: AttemptStore,
    run_store: RunStore,
}

impl DispatchManager {
    pub fn new(
        cell_id: u32,
        run_store: RunStore,
        attempt_store: AttemptStore,
    ) -> Self {
        Self {
            _cell_id: cell_id,
            run_store,
            attempt_store,
        }
    }

    pub async fn start(&self) -> Result<(), DispatcherManagerError> {
        // TODO: Fetch only runs for this cell
        let pending_runs = self
            .run_store
            .get_runs_by_status(RunStatus::Attempting)
            .await?;

        info!(
            "Loaded {} pending runs from the database",
            pending_runs.len()
        );

        for r in pending_runs {
            tokio::spawn(
                RunJob::from(
                    r,
                    self.run_store.clone(),
                    self.attempt_store.clone(),
                )
                .run(),
            );
        }

        Ok(())
    }

    pub async fn run(
        &self,
        run: Run,
        mode: DispatchMode,
    ) -> Result<Run, DispatcherManagerError> {
        self.run_store.store_run(run.clone()).await?;

        let run_job = RunJob::from(
            run,
            self.run_store.clone(),
            self.attempt_store.clone(),
        );

        Ok(match mode {
            | DispatchMode::Unknown => {
                panic!("Unknown dispatch mode");
            }
            | DispatchMode::Async => {
                // Cloning here to avoid unnecessary clones if sync.
                let run = run_job.run.clone();
                tokio::spawn(async move { run_job.run().await });
                run
            }
            | DispatchMode::Sync => run_job.run().await,
        })
    }
}

pub struct RunJob {
    pub run: Run,
    run_store: RunStore,
    attempt_store: AttemptStore,
}

impl RunJob {
    fn from(
        run: Run,
        run_store: RunStore,
        attempt_store: AttemptStore,
    ) -> Self {
        Self {
            run,
            run_store,
            attempt_store,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn run(self) -> Run {
        increment_gauge!("dispatcher.inflight_runs_total", 1.0);
        debug_assert!(self.run.status == RunStatus::Attempting);
        let run = match &self.run.action {
            | Action::Webhook(_) => {
                let e = WebhookActionJob {
                    run: self.run,
                    run_store: self.run_store.clone(),
                    attempt_store: self.attempt_store,
                };
                e.run().await
            }
        };
        decrement_gauge!("dispatcher.inflight_runs_total", 1.0);
        let total_duration_s = Utc::now()
            .signed_duration_since(run.created_at)
            .to_std()
            .unwrap_or_else(|_| Duration::default())
            .as_secs_f64();

        if run.status == RunStatus::Failed {
            e!(
                project_id = run.project_id,
                RunFailed {
                    meta: run.meta().into(),
                    total_duration_s,
                    latest_attempt_id: run
                        .latest_attempt_id
                        .as_ref()
                        .cloned()
                        .map(Into::into),
                }
            );
        } else if run.status == RunStatus::Succeeded {
            e!(
                project_id = run.project_id,
                RunSucceeded {
                    meta: run.meta().into(),
                    total_duration_s,
                    latest_attempt_id: run
                        .latest_attempt_id
                        .as_ref()
                        .cloned()
                        .map(Into::into),
                }
            );
        }
        run
    }
}
