use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use lib::database::trigger_store::{TriggerStore, TriggerStoreError};
use lib::grpc_client_provider::DispatcherClientProvider;
use lib::model::ValidShardedId;
use lib::service::ServiceContext;
use lib::types::{ProjectId, Run, Status, Trigger, TriggerId, TriggerManifest};
use proto::scheduler_proto::{InstallTriggerRequest, InstallTriggerResponse};
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
    pub async fn update_trigger(
        &self,
        mut existing_trigger: Trigger,
        install_trigger: InstallTriggerRequest,
    ) -> Result<InstallTriggerResponse, TriggerError> {
        // TODO: Take snapshots of previous triggers and store for auditing.
        if install_trigger.fail_if_exists {
            return Err(TriggerError::UpdateNotAllowed(existing_trigger.id));
        }

        // Is the updated trigger a "Scheduled" trigger?
        // Do we have the existing trigger in memory?
        let trigger_id = existing_trigger.id.clone();
        let triggers_map = self.triggers.clone();
        // Why clone? because we might need it if the trigger was not in the
        // active map.
        let install_cloned = install_trigger.clone();
        let mut updated_trigger = tokio::task::spawn_blocking(move || {
            let mut w = triggers_map.write().unwrap();
            let Some(alive_trigger) = w.get(&trigger_id) else {
                return None;
            };
            // are we attempting to install a `scheduled` replacement, or is
            // the replacement an on_demand one?
            let mut updated_trigger = alive_trigger.clone();
            updated_trigger.update(
                install_cloned.name,
                install_cloned.description,
                install_cloned.reference,
                install_cloned.payload.map(Into::into),
                install_cloned.schedule.map(Into::into),
                install_cloned.action.unwrap().into(),
            );
            if updated_trigger.schedule.is_some() {
                Some(w.add_or_update(
                    updated_trigger,
                    // We fast forward on update to avoid triggering old
                    // events if the new trigger
                    // has any old timestamps.
                    /* fast_forward = */
                    true,
                ))
            } else {
                // If the old is scheduled and the new is not,
                // we are in trouble, so we remove it from
                // the map.
                w.pop_trigger(&trigger_id);
                Some(Ok(updated_trigger))
            }
        })
        .await?
        .transpose()?;

        // if we don't have an updated_trigger, it means that we can merge with
        // the `existing_trigger` to get the updated one.
        if updated_trigger.is_none() {
            existing_trigger.update(
                install_trigger.name,
                install_trigger.description,
                install_trigger.reference,
                install_trigger.payload.map(|p| p.into()),
                install_trigger.schedule.map(|s| s.into()),
                install_trigger.action.unwrap().into(),
            );
            updated_trigger = Some(existing_trigger);
        }

        // Guaranteed to be set at this point.
        let updated_trigger = updated_trigger.unwrap();

        // We have the updated trigger, let's save it to the database.
        self.store.update_trigger(&updated_trigger).await?;

        //
        // RESPOND
        let reply = InstallTriggerResponse {
            trigger: Some(updated_trigger.into()),
            already_existed: true,
        };
        Ok(reply)
    }

    pub async fn install_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        mut install_trigger: InstallTriggerRequest,
    ) -> Result<InstallTriggerResponse, TriggerError> {
        // If we have an Id already, we must allow updates.
        if install_trigger.id.is_some() && install_trigger.fail_if_exists {
            return Err(TriggerError::UpdateNotAllowed(
                install_trigger.id.unwrap().into(),
            ));
        }

        // Reset remaining if it was set.
        // TODO: When updating, allow the user to express their intent to
        // whether they want to reset the remaining or not.
        if let Some(schedule) = install_trigger.schedule.as_mut() {
            // the inner .schedule must be set at this point.
            // The spinner will update `remaining` to the accurate value as soon
            // as it runs.
            let schedule = schedule.schedule.as_mut().unwrap();
            match schedule {
                | proto::trigger_proto::schedule::Schedule::Recurring(r)
                    if r.limit.is_some() =>
                {
                    r.remaining = r.limit;
                }
                | proto::trigger_proto::schedule::Schedule::Recurring(r) => {
                    r.remaining = None;
                }
                | proto::trigger_proto::schedule::Schedule::RunAt(r) => {
                    r.remaining = Some(r.timepoints.len() as u64);
                }
            };
        }

        // ** Are we installing new or updating an existing trigger? **
        //
        // find the existing trigger by id
        if let Some(trigger_id) = install_trigger.id.clone() {
            let trigger_id = TriggerId::from(trigger_id);
            let existing_trigger =
                self.store.get_trigger(&project, &trigger_id).await?;
            let Some(existing_trigger) = existing_trigger else {
                return Err(TriggerError::NotFound(trigger_id));
            };
            return self
                .update_trigger(existing_trigger, install_trigger)
                .await;
        }

        let id = TriggerId::generate(&project);

        // We want to keep a copy in case we have to call update later.
        let copied_request = install_trigger.clone();
        let is_scheduled = install_trigger.schedule.is_some();
        let trigger = Trigger {
            id: id.into(),
            project: project.clone(),
            reference: install_trigger.reference.clone(),
            name: install_trigger.name,
            description: install_trigger.description,
            created_at: Utc::now(),
            updated_at: None,
            action: install_trigger.action.unwrap().into(),
            payload: install_trigger.payload.map(|p| p.into()),
            schedule: install_trigger.schedule.map(|s| s.into()),
            status: if is_scheduled {
                Status::Scheduled
            } else {
                Status::OnDemand
            },
            last_ran_at: None,
        };

        let store_result = self.store.install_trigger(&trigger).await;

        match store_result {
            | Ok(_) => {}
            | Err(TriggerStoreError::DuplicateRecord) => {
                // Do we have an existing trigger with this reference?
                // if we search first, then we might end up inserting twice and
                // failing with duplicate error to the user. If
                // we always attempt to insert and fallback to
                // update, we won't produce duplicate
                // errors unless the user asks to fail if exists explicitly.
                if copied_request.reference.is_some()
                    && !copied_request.fail_if_exists
                {
                    let mut triggers = self
                        .store
                        .get_triggers_by_project(
                            &project.clone(),
                            copied_request.reference.clone(),
                            /* statuses = */ None,
                            /* before = */ None,
                            /* after = */ None,
                            /* limit = */ 1,
                        )
                        .await?;
                    if let Some(trigger) = triggers.pop() {
                        return self
                            .update_trigger(trigger, copied_request)
                            .await;
                    }
                }
                return Err(TriggerError::AlreadyExists(
                    trigger.reference.unwrap(),
                ));
            }
            | Err(e) => return Err(e.into()),
        };

        let trigger = if is_scheduled {
            // We only install scheduled triggers in the ActiveMap
            let triggers = self.triggers.clone();
            tokio::task::spawn_blocking(move || {
                let mut w = triggers.write().unwrap();
                w.add_or_update(trigger, /* fast_forward = */ false)
            })
            .await?
        } else {
            Ok(trigger)
        }?;

        let reply = InstallTriggerResponse {
            trigger: Some(trigger.into()),
            already_existed: false,
        };
        Ok(reply)
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
                    .get_trigger(&project, &id)
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
        statuses: Option<Vec<Status>>,
        limit: usize,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
    ) -> Result<Vec<TriggerManifest>, TriggerError> {
        // Hopefully in the future we will be able to get the manifest directly
        // from database instead of fetching the entire trigger.
        let triggers = self
            .store
            .get_triggers_by_project(
                &project, reference, statuses, before, after, limit,
            )
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
    pub async fn run_trigger(
        &self,
        project: ValidShardedId<ProjectId>,
        id: TriggerId,
        mode: DispatchMode,
    ) -> Result<Run, TriggerError> {
        let trigger = self.get_trigger(project, id).await?;

        if trigger.status == Status::Cancelled {
            return Err(TriggerError::InvalidStatus(
                "run".to_string(),
                trigger.status,
            ));
        }
        let run =
            dispatch(trigger, self.dispatcher_client_provider.clone(), mode)
                .await?;
        Ok(run)
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
                Status::Scheduled.as_operation(),
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
                "Skipping missed runs, the scheduler will ignore the \
                 last_ran_at of all triggers. This will cause triggers to \
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
