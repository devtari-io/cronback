use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use lib::database::trigger_store::{TriggerStore, TriggerStoreError};
use lib::grpc_client_provider::DispatcherClientProvider;
use lib::model::ValidShardedId;
use lib::service::ServiceContext;
use lib::types::{
    Invocation,
    ProjectId,
    Status,
    Trigger,
    TriggerId,
    TriggerManifest,
};
use proto::scheduler_proto::InstallTriggerRequest;
use tracing::{debug, error, info, trace, warn};

use super::dispatch::dispatch;
use super::event_dispatcher::DispatchMode;
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
        trace!("Attempting to checkpoint triggers");
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
            let res = self.store.update_trigger(&trigger).await;
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
        project: ValidShardedId<ProjectId>,
        install_trigger: InstallTriggerRequest,
    ) -> Result<Trigger, TriggerError> {
        let id = TriggerId::generate(&project);

        let is_scheduled = install_trigger.schedule.is_some();
        let trigger = Trigger {
            id: id.into(),
            project,
            reference: install_trigger.reference,
            name: install_trigger.name,
            description: install_trigger.description,
            created_at: Utc::now(),
            emit: install_trigger.emit.into_iter().map(|e| e.into()).collect(),
            payload: install_trigger.payload.map(|p| p.into()),
            schedule: install_trigger.schedule.map(|s| s.into()),
            status: if is_scheduled {
                Status::Active
            } else {
                Status::OnDemand
            },
            last_invoked_at: None,
        };

        let store_result = self.store.install_trigger(&trigger).await;

        match store_result {
            | Ok(_) => {}
            | Err(TriggerStoreError::DuplicateRecord) => {
                return Err(TriggerError::AlreadyExists(
                    trigger.reference.unwrap(),
                ));
            }
            | Err(e) => return Err(e.into()),
        };

        if is_scheduled {
            // We only install scheduled triggers in the ActiveMap
            let triggers = self.triggers.clone();
            tokio::task::spawn_blocking(move || {
                let mut w = triggers.write().unwrap();
                w.add_or_update(trigger, /* fast_forward = */ false)
            })
            .await?
        } else {
            Ok(trigger)
        }
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn get_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        id: TriggerId,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        let inner_id = id.clone();
        let trigger_res = tokio::task::spawn_blocking(move || {
            let r = triggers.read().unwrap();
            r.get(&inner_id)
                .ok_or_else(|| TriggerError::NotFound(inner_id))
                .cloned()
        })
        .await?;
        // Get from the database if this is not an active trigger.
        match trigger_res {
            | Ok(trigger) if trigger.project == project => Ok(trigger),
            // The trigger was found but owned by a different user!
            | Ok(_) => Err(TriggerError::NotFound(id.clone())),
            | Err(TriggerError::NotFound(_)) => {
                self.store
                    .get_trigger(&id)
                    .await?
                    .ok_or_else(|| TriggerError::NotFound(id))
            }
            | Err(e) => Err(e),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_triggers(
        &self,
        project: ValidShardedId<ProjectId>,
        reference: Option<String>,
        limit: usize,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
    ) -> Result<Vec<TriggerManifest>, TriggerError> {
        // Hopefully in the future we will be able to get the manifest directly
        // from database instead of fetching the entire trigger.
        let triggers = self
            .store
            .get_triggers_by_project(&project, reference, before, after, limit)
            .await?;

        let (alive, dead): (Vec<_>, Vec<_>) =
            triggers.into_iter().partition(Trigger::alive);

        // Swap live triggers with ones from the scheduler in-memory state.
        let active_trigger_map = self.triggers.clone();
        let alive_triggers: Vec<TriggerManifest> =
            tokio::task::spawn_blocking(move || {
                let r = active_trigger_map.read().unwrap();
                alive
                    .into_iter()
                    .filter_map(|t| r.get(&t.id))
                    .map(Trigger::get_manifest)
                    .collect()
            })
            .await?;

        let mut results: Vec<TriggerManifest> =
            Vec::with_capacity(dead.len() + alive_triggers.len());

        // merge live and dead
        results.extend(dead.into_iter().map(Trigger::into_manifest));
        results.extend(alive_triggers);
        // reverse sort the list by created_at (newer first)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(results)
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn invoke_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        id: TriggerId,
        mode: DispatchMode,
    ) -> Result<Invocation, TriggerError> {
        let trigger = self.get_trigger(project, id).await?;

        if trigger.status == Status::Cancelled {
            return Err(TriggerError::InvalidStatus(
                "invoke".to_string(),
                trigger.status,
            ));
        }
        let invocation =
            dispatch(trigger, self.dispatcher_client_provider.clone(), mode)
                .await?;
        Ok(invocation)
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn pause_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        id: TriggerId,
    ) -> Result<TriggerManifest, TriggerError> {
        let triggers = self.triggers.clone();
        let status = self.get_trigger_status(&project, &id).await?;
        // if value, check that it's alive.
        if !status.alive() {
            return Err(TriggerError::InvalidStatus(
                Status::Paused.as_operation(),
                status,
            ));
        }
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.pause(&id).map(|_| w.get(&id).unwrap().get_manifest())
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn resume_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        id: TriggerId,
    ) -> Result<TriggerManifest, TriggerError> {
        let triggers = self.triggers.clone();
        let status = self.get_trigger_status(&project, &id).await?;
        // if value, check that it's alive.
        if !status.alive() {
            return Err(TriggerError::InvalidStatus(
                Status::Active.as_operation(),
                status,
            ));
        }
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.resume(&id).map(|_| w.get(&id).unwrap().get_manifest())
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_id = %id))]
    pub async fn cancel_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        id: TriggerId,
    ) -> Result<TriggerManifest, TriggerError> {
        let triggers = self.triggers.clone();
        let status = self.get_trigger_status(&project, &id).await?;
        // if value, check that it's alive.
        if !status.cancelleable() {
            return Err(TriggerError::InvalidStatus(
                Status::Cancelled.as_operation(),
                status,
            ));
        }

        if status == Status::OnDemand {
            let mut trigger = self.get_trigger(project, id).await?;
            trigger.status = Status::Cancelled;
            let manifest = trigger.get_manifest();
            self.store.update_trigger(&trigger).await?;
            Ok(manifest)
        } else {
            let inner_id = id.clone();
            tokio::task::spawn_blocking(move || {
                let mut w = triggers.write().unwrap();
                // We blindly cancel here since we have checked earlier that
                // this trigger is owned by the right project.
                // Otherwise, we won't reach this line.
                w.cancel(&inner_id)
                    .map(|_| w.get(&inner_id).unwrap().get_manifest())
            })
            .await?
        }
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

    async fn get_trigger_status(
        &self,
        project: &ValidShardedId<ProjectId>,
        trigger_id: &TriggerId,
    ) -> Result<Status, TriggerError> {
        let status = self.store.get_status(project, trigger_id).await?;
        // if None -> Trigger doesn't exist
        status.ok_or_else(|| TriggerError::NotFound(trigger_id.clone()))
    }
}
