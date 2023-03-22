use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use proto::scheduler_proto::InstallTriggerRequest;
use shared::database::trigger_store::TriggerStore;
use shared::grpc_client_provider::DispatcherClientProvider;
use shared::service::ServiceContext;
use shared::types::{Invocation, OwnerId, Status, Trigger, TriggerId};
use tracing::{debug, error, info, warn};

use super::dispatch::dispatch;
use super::spinner::{Spinner, SpinnerHandle};
use super::triggers::{ActiveTriggerMap, TriggerError};
use crate::sched::triggers::ActiveTrigger;

/**
 *
 * EventScheduler is the gateway to the scheduling and dispatch thread
 * (spinner) It's designed to be easily shared with inner mutability and
 * minimal locking to reduce contention.
 *
 *
 * Event Scheduler also wraps the database. I'll load and store triggers
 * from the database as needed. Installing a new trigger happens on two
 * steps:
 * - Inserting the trigger in the database
 * - Adding the trigger to the ActiveTriggerMap (while holding a write lock)
 *
 * This is designed like this to minimise locking the ActiveTriggerMap (vs.
 * making database queries from the TriggerMap while holding the write lock
 * unnecessarily)
 *
 * EventScheduler owns
 *   - Active TriggerMap
 *   - The SpinnerHandle
 *   - Database Handle
 * Provides:
 *   - API to install/remove/update/query triggers
 *   - Start/pause the spinner.
 *
 * Needs to take care of:
 *   - Compaction/eviction: remove expired triggers.
 *   - Flushes active triggers map into the database periodically.
 */
pub(crate) struct EventScheduler {
    context: ServiceContext,
    triggers: Arc<RwLock<ActiveTriggerMap>>,
    spinner: Mutex<Option<SpinnerHandle>>,
    store: Box<dyn TriggerStore + Send + Sync>,
    dispatcher_client_provider: Arc<DispatcherClientProvider>,
}

impl EventScheduler {
    pub fn new(
        context: ServiceContext,
        store: Box<dyn TriggerStore + Send + Sync>,
        dispatcher_client_provider: Arc<DispatcherClientProvider>,
    ) -> Self {
        Self {
            context,
            triggers: Arc::default(),
            spinner: Mutex::default(),
            store,
            dispatcher_client_provider,
        }
    }

    pub async fn start(&self) -> Result<(), TriggerError> {
        {
            let mut spinner = self.spinner.lock().unwrap();
            if spinner.is_some() {
                info!("EventScheduler has already started!");
                return Ok(());
            }
            *spinner = Some(
                Spinner::new(
                    self.context.clone(),
                    self.triggers.clone(),
                    self.dispatcher_client_provider.clone(),
                )
                .start(),
            );
        }

        self.load_triggers_from_database().await
    }

    // Do things like compaction and checkpointing.
    pub async fn perform_checkpoint(&self) {
        debug!("Attempting to checkpoint triggers");
        // Persist active triggers to database
        let triggers = self.triggers.clone();
        // Triggers that have been cancelled or expired.
        let mut retired_triggers: HashMap<TriggerId, ActiveTrigger> =
            HashMap::new();
        let mut triggers_to_save = Vec::new();
        {
            // Write lock held
            let mut w = triggers.write().unwrap();
            let triggers_pending = w.awaiting_db_flush();
            if !triggers_pending.is_empty() {
                info!(
                    "Checkpointing {} triggers to database",
                    triggers_pending.len()
                );
            }

            for trigger_id in triggers_pending {
                let Some(trigger) = w.get(&trigger_id) else {
                    continue;
                };
                // We can't hold the lock in async
                // scope so we need to collect the triggers to
                // save and then save them outside the lock.
                triggers_to_save.push(trigger.clone());
                // A trigger can be removed from the active map if it is
                // expired/cancelled
                if w.is_trigger_retired(&trigger_id) {
                    debug!(
                        "Trigger {} is retired and will be removed from \
                         spinner",
                        trigger_id
                    );
                    retired_triggers.insert(
                        trigger_id.clone(),
                        w.pop_trigger(&trigger_id).unwrap(),
                    );
                }
            }
            // reset awaiting db flush set
            w.clear_db_flush();
        }

        // Save to database.
        let mut failed = Vec::new();
        for trigger in triggers_to_save {
            debug!(trigger_id = %trigger.id, "Checkpointing trigger");
            // TODO: Consider batch-inserting.
            let res = self.store.install_trigger(&trigger).await;
            if let Err(e) = res {
                error!(
                    trigger_id = %trigger.id,
                    "Failed to checkpoint trigger: {}, will be retried on next checkpoint",
                    e
                );
                failed.push(trigger.id);
            }
        }
        // Failed triggers will be retried on next checkpoint.
        {
            let mut w = triggers.write().unwrap();
            for trigger_id in failed {
                // expired/cancelled triggers were removed, if we failed to
                // flush we should put them back.
                if retired_triggers.contains_key(&trigger_id) {
                    w.push_trigger(
                        trigger_id.clone(),
                        retired_triggers.remove(&trigger_id).unwrap(),
                    )
                }
                w.add_to_awaiting_db_flush(trigger_id);
            }
        }

        // TODO: Expired triggers should be removed from active map.
    }

    #[tracing::instrument(skip_all)]
    pub async fn install_trigger(
        &self,
        install_trigger: InstallTriggerRequest,
    ) -> Result<Trigger, TriggerError> {
        let id = TriggerId::new(&OwnerId(install_trigger.owner_id.clone()));

        let trigger = Trigger {
            id,
            owner_id: install_trigger.owner_id.into(),
            reference_id: install_trigger.reference_id,
            name: install_trigger.name,
            description: install_trigger.description,
            created_at: Utc::now(),
            emit: install_trigger.emit.into_iter().map(|e| e.into()).collect(),
            payload: install_trigger.payload.unwrap().into(),
            schedule: install_trigger.schedule.map(|s| s.into()),
            status: Status::Active,
            hidden_last_invoked_at: None,
        };

        self.store.install_trigger(&trigger).await?;

        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.add_or_update(trigger, /* fast_forward = */ false)
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn get_trigger(
        &self,
        id: TriggerId,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let r = triggers.read().unwrap();
            r.get(&id)
                .ok_or_else(|| TriggerError::NotFound(id))
                .cloned()
        })
        .await?
        // TODO: Get from the database if this is not an active trigger.
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn invoke_trigger(
        &self,
        id: TriggerId,
    ) -> Result<Invocation, TriggerError> {
        let trigger = self.store.get_trigger(&id).await?;
        let Some(trigger) = trigger else {
            return Err(TriggerError::NotFound(id));
        };
        let invocation = dispatch(
            trigger,
            self.dispatcher_client_provider.clone(),
            super::event_dispatcher::DispatchMode::Sync,
        )
        .await?;
        Ok(invocation)
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn pause_trigger(
        &self,
        id: TriggerId,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.pause(&id).map(|_| w.get(&id).unwrap().clone())
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn resume_trigger(
        &self,
        id: TriggerId,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.resume(&id).map(|_| w.get(&id).unwrap().clone())
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn cancel_trigger(
        &self,
        id: TriggerId,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.cancel(&id).map(|_| w.get(&id).unwrap().clone())
        })
        .await?
    }

    pub async fn shutdown(&self) {
        {
            let mut spinner = self.spinner.lock().unwrap();
            // will drop the current spinner after shutdown.
            if let Some(spinner) = spinner.take() {
                spinner.shutdown();
            } else {
                info!("EventScheduler has already been shutdown!");
            }
        }
        self.perform_checkpoint().await;
        let mut triggers = self.triggers.write().unwrap();
        triggers.clear();
    }

    async fn load_triggers_from_database(&self) -> Result<(), TriggerError> {
        let triggers = self.store.get_all_active_triggers().await?;

        info!("Found {} active triggers in the database", triggers.len());
        let config = self.context.get_config();
        if config.scheduler.dangerous_fast_forward {
            warn!(
                "Skipping missed invocations, the scheduler will ignore the \
                 last_invoked_at of all triggers. This will cause triggers to \
                 execute future events only"
            );
        }
        let mut map = self.triggers.write().unwrap();
        for trigger in triggers {
            map.add_or_update(
                trigger,
                config.scheduler.dangerous_fast_forward,
            )?;
        }
        Ok(())
    }
}
