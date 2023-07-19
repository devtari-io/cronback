use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use async_recursion::async_recursion;
use chrono::Utc;
use lib::clients::dispatcher_client::ScopedDispatcherClient;
use lib::database::pagination::PaginatedResponse;
use lib::e;
use lib::grpc_client_provider::GrpcClientProvider;
use lib::model::ValidShardedId;
use lib::prelude::*;
use lib::service::ServiceContext;
use lib::types::{ProjectId, TriggerId};
use proto::common::request_precondition::PreconditionType;
use proto::common::{PaginationIn, UpsertEffect};
use proto::events::TriggerMeta;
use proto::run_proto::Run;
use proto::scheduler_proto::{UpsertTriggerRequest, UpsertTriggerResponse};
use tracing::{debug, error, info, trace, warn};

use super::active_triggers::{ActiveTrigger, ActiveTriggerMap};
use super::dispatch::{dispatch, DispatchMode};
use super::spinner::{Spinner, SpinnerHandle};
use crate::db_model::triggers::Status;
use crate::db_model::Trigger;
use crate::error::TriggerError;
use crate::trigger_store::{TriggerStore, TriggerStoreError};

///  SpinnerController is the gateway to the scheduling and dispatch thread
///  (spinner) It's designed to be easily shared with inner mutability and
///  minimal locking to reduce contention.
///
///
///  SpinnerController also wraps the database. I'll load and store triggers
///  from the database as needed. Installing a new trigger happens on two
///  steps:
///  - Inserting the trigger in the database
///  - Adding the trigger to the ActiveTriggerMap (while holding a write lock)
///
///  This is designed like this to minimise locking the ActiveTriggerMap (vs.
///  making database queries from the TriggerMap while holding the write lock
///  unnecessarily)
///
///  SpinnerController owns
///  - Active TriggerMap
///  - The SpinnerHandle
///  - Database Handle
///  Provides:
///  - API to install/remove/update/query triggers
///  - Start/pause the spinner.
///
///  Needs to take care of:
///  - Compaction/eviction: remove expired triggers.
///  - Flushes active triggers map into the database periodically.
pub(crate) struct SpinnerController {
    context: ServiceContext,
    triggers: Arc<RwLock<ActiveTriggerMap>>,
    spinner: Mutex<Option<SpinnerHandle>>,
    store: Box<dyn TriggerStore + Send + Sync>,
    trigger_name_cache: Arc<RwLock<HashMap<String, TriggerId>>>,
    dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherClient>>,
}

impl SpinnerController {
    pub fn new(
        context: ServiceContext,
        store: Box<dyn TriggerStore + Send + Sync>,
        dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherClient>>,
    ) -> Self {
        Self {
            context,
            triggers: Arc::default(),
            spinner: Mutex::default(),
            trigger_name_cache: Arc::default(),
            store,
            dispatcher_clients,
        }
    }

    pub async fn start(&self) -> Result<(), TriggerError> {
        {
            let mut spinner = self.spinner.lock().unwrap();
            if spinner.is_some() {
                info!("SpinnerController has already started!");
                return Ok(());
            }
            *spinner = Some(
                Spinner::new(
                    self.context.clone(),
                    self.triggers.clone(),
                    self.dispatcher_clients.clone(),
                )
                .start(),
            );
        }

        self.load_triggers_from_database().await
    }

    /// Checkpointing flushes the dirty active triggers to the database
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
                // We can't hold the lock in async scope so we need to collect
                // the triggers to save and then save them outside the lock.
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
            let res = self.store.update_trigger(trigger.clone()).await;
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
    }

    pub async fn get_trigger_id_opt(
        &self,
        project_id: &ProjectId,
        name: &str,
    ) -> Result<Option<TriggerId>, TriggerStoreError> {
        // find the id from the cache, if we can't find it, query from the
        // database. We only acquire write lock when we update the
        // cache.
        {
            let name_cache = self.trigger_name_cache.read().unwrap();
            if let Some(trigger_id) = name_cache.get(name) {
                info!(
                    trigger_id = ?trigger_id,
                    project_id = ?project_id,
                    name = name,
                    "Found trigger with name cache"
                );
                return Ok(Some(trigger_id.clone()));
            }
        }
        // Cache miss.
        let trigger_id = self
            .store
            .find_trigger_id_for_name(project_id, name)
            .await?;
        if let Some(trigger_id) = trigger_id {
            self.trigger_name_cache
                .write()
                .unwrap()
                .insert(name.to_string(), trigger_id.clone());
            Ok(Some(trigger_id))
        } else {
            Ok(None)
        }
    }

    fn remove_name_from_cache(&self, name: &str) {
        let mut name_cache = self.trigger_name_cache.write().unwrap();
        name_cache.remove(name);
    }

    pub async fn get_trigger_id(
        &self,
        project_id: &ProjectId,
        name: &str,
    ) -> Result<TriggerId, TriggerError> {
        let id = self.get_trigger_id_opt(project_id, name).await?;
        if let Some(id) = id {
            Ok(id)
        } else {
            Err(TriggerError::NotFound(name.to_string()))
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn update_trigger(
        &self,
        _existing_trigger: Trigger,
        _upsert_request: UpsertTriggerRequest,
    ) -> Result<UpsertTriggerResponse, TriggerError> {
        todo!()
        // TODO: Take snapshots of previous triggers and store for auditing.
        // if upsert_request.fail_if_exists {
        //     return Err(TriggerError::UpdateNotAllowed(existing_trigger.id));
        // }
        //
        // // Is the updated trigger a "Scheduled" trigger?
        // // Do we have the existing trigger in memory?
        // let trigger_id = existing_trigger.id.clone();
        // let triggers_map = self.triggers.clone();
        // // Why clone? because we might need it if the trigger was not in the
        // // active map.
        // let install_cloned = upsert_request.clone();
        // let mut updated_trigger = tokio::task::spawn_blocking(move || {
        //     let mut w = triggers_map.write().unwrap();
        //     let Some(alive_trigger) = w.get(&trigger_id) else {
        //         return None;
        //     };
        //     // are we attempting to install a `scheduled` replacement, or is
        //     // the replacement an on_demand one?
        //     let mut updated_trigger = alive_trigger.clone();
        //     updated_trigger.update(
        //         install_cloned.name,
        //         install_cloned.description,
        //         install_cloned.reference,
        //         install_cloned.payload.map(Into::into),
        //         install_cloned.schedule.map(Into::into),
        //         install_cloned.action.unwrap().into(),
        //     );
        //     if updated_trigger.schedule.is_some() {
        //         Some(w.add_or_update(
        //             updated_trigger,
        //             // We fast forward on update to avoid triggering old
        //             // events if the new trigger
        //             // has any old timestamps.
        //             /* fast_forward = */
        //             true,
        //         ))
        //     } else {
        //         // If the old is scheduled and the new is not,
        //         // we are in trouble, so we remove it from
        //         // the map.
        //         w.pop_trigger(&trigger_id);
        //         Some(Ok(updated_trigger))
        //     }
        // })
        // .await?
        // .transpose()?;
        //
        // // if we don't have an updated_trigger, it means that we can merge
        // with // the `existing_trigger` to get the updated one.
        // if updated_trigger.is_none() {
        //     existing_trigger.update(
        //         install_trigger.name,
        //         install_trigger.description,
        //         install_trigger.reference,
        //         install_trigger.payload.map(|p| p.into()),
        //         install_trigger.schedule.map(|s| s.into()),
        //         install_trigger.action.unwrap().into(),
        //     );
        //     updated_trigger = Some(existing_trigger);
        // }
        //
        // // Guaranteed to be set at this point.
        // let updated_trigger = updated_trigger.unwrap();
        //
        // // We have the updated trigger, let's save it to the database.
        // self.store.update_trigger(updated_trigger.clone()).await?;
        //
        // //
        // // RESPOND
        // let reply = UpsertTriggerResponse {
        //     trigger: Some(updated_trigger.into()),
        //     already_existed: true,
        // };
        // Ok(reply)
    }

    #[async_recursion]
    pub async fn upsert_trigger(
        &self,
        context: RequestContext,
        upsert_request: UpsertTriggerRequest,
    ) -> Result<UpsertTriggerResponse, TriggerError> {
        // We assume that trigger will always be set.
        let project_id = &context.project_id;
        let mut trigger = upsert_request.trigger.clone().unwrap();

        // Reset remaining if it was set.
        // TODO: When updating, allow the user to express their intent to
        // whether they want to reset the remaining or not.
        if let Some(schedule) = trigger.schedule.as_mut() {
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

        let request_precondition = upsert_request.precondition.get_or_default();
        // ** Are we installing new or updating an existing trigger? **
        //
        // find the existing trigger by name
        assert!(!trigger.name.is_empty());
        let trigger_name = trigger.name.as_ref();
        let existing_trigger = self
            .store
            .get_trigger_by_name(project_id, trigger_name)
            .await?;
        // We already have an existing trigger with the same name and the
        // user asked us to fail if exists.
        if request_precondition.precondition_type()
            == PreconditionType::MustNotExist
            && existing_trigger.is_some()
        {
            return Err(TriggerError::AlreadyExists(trigger_name.to_string()));
        } else if let Some(existing_trigger) = existing_trigger {
            // Update
            return self.update_trigger(existing_trigger, upsert_request).await;
        }
        // It doesn't exist and we are not updating, so we are installing.
        //
        assert!(existing_trigger.is_none());

        let id = TriggerId::generate(project_id);

        // We want to keep a copy in case we have to call update later.
        //let copied_request = install_trigger.clone();
        let is_scheduled = trigger.schedule.is_some();
        let trigger = Trigger {
            id: id.into(),
            project_id: project_id.clone(),
            name: trigger_name.to_string(),
            description: trigger.description,
            created_at: Utc::now(),
            updated_at: None,
            action: trigger.action.unwrap().into(),
            payload: trigger.payload.map(|p| p.into()),
            schedule: trigger.schedule.map(|s| s.into()),
            status: if is_scheduled {
                Status::Scheduled
            } else {
                Status::OnDemand
            },
            last_ran_at: None,
        };

        let store_result = self.store.install_trigger(trigger.clone()).await;

        match store_result {
            | Ok(_) => { /* success */ }
            | Err(TriggerStoreError::DuplicateRecord) => {
                // Quite possibly a race with another install.
                // Do we have an existing trigger with this name?
                // if we search first, then we might end up inserting twice and
                // failing with duplicate error to the user. If
                // we always attempt to insert and fallback to
                // update, we won't produce duplicate
                // errors unless the user asks to fail if exists explicitly.
                if request_precondition.precondition_type()
                    != PreconditionType::MustNotExist
                {
                    // try again after 250ms.
                    info!(
                        "Hit a race while trying to install the trigger \
                         {trigger_name} in project
                    {project_id}. Will retry after 250ms!"
                    );
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    return self.upsert_trigger(context, upsert_request).await;
                }
                return Err(TriggerError::AlreadyExists(
                    trigger_name.to_string(),
                ));
            }
            | Err(e) => return Err(e.into()),
        };

        // Trigger installed, now we need that to be reflected in the active
        // map (if needed). We only install scheduled triggers in the ActiveMap
        let trigger = if is_scheduled {
            let triggers = self.triggers.clone();
            tokio::task::spawn_blocking(move || {
                let mut w = triggers.write().unwrap();
                w.add_or_update(trigger, /* fast_forward = */ false)
            })
            .await?
        } else {
            Ok(trigger)
        }?;

        e!(
            context = context,
            TriggerCreated {
                meta: trigger.meta().into(),
            }
        );

        let reply = UpsertTriggerResponse {
            trigger: Some(trigger.into()),
            effect: UpsertEffect::Created.into(),
        };
        Ok(reply)
    }

    #[tracing::instrument(skip_all, fields(trigger_name = %name, project_id = %context.project_id))]
    pub async fn get_trigger(
        &self,
        context: RequestContext,
        name: String,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        let trigger_id =
            self.get_trigger_id(&context.project_id, &name).await?;

        let cloned_name = name.clone();
        let trigger_res = tokio::task::spawn_blocking(move || {
            let r = triggers.read().unwrap();
            r.get(&trigger_id)
                .ok_or_else(|| TriggerError::NotFound(cloned_name))
                .cloned()
        })
        .await?;
        // Get from the database if this is not an active trigger.
        match trigger_res {
            // We must validate project ownership since we didn't check that
            // when retrieving from active map.
            | Ok(trigger) if trigger.project_id == context.project_id => {
                Ok(trigger)
            }
            // The trigger was found but owned by a different user!
            | Ok(_) => Err(TriggerError::NotFound(name.to_string())),
            | Err(TriggerError::NotFound(_)) => {
                self.store
                    .get_trigger_by_name(&context.project_id, &name)
                    .await?
                    .ok_or_else(|| TriggerError::NotFound(name.to_string()))
            }
            | Err(e) => Err(e),
        }
    }

    #[tracing::instrument(skip_all, fields(project_id = %context.project_id))]
    pub async fn list_triggers(
        &self,
        context: RequestContext,
        statuses: Option<Vec<Status>>,
        pagination: PaginationIn,
    ) -> Result<PaginatedResponse<Trigger>, TriggerError> {
        // Hopefully in the future we will be able to get the compact version
        // directly from database instead of fetching the entire trigger.
        let paginated_triggers = self
            .store
            .get_triggers_by_project(&context.project_id, pagination, statuses)
            .await?;

        let PaginatedResponse { data, pagination } = paginated_triggers;

        let (alive, dead): (Vec<_>, Vec<_>) =
            data.into_iter().partition(Trigger::alive);

        // Swap live triggers with ones from the scheduler in-memory state.
        let active_trigger_map = self.triggers.clone();
        let alive_triggers: Vec<Trigger> =
            tokio::task::spawn_blocking(move || {
                let r = active_trigger_map.read().unwrap();
                alive
                    .into_iter()
                    // If we can't find it in scheduler in-memory state, just
                    // return what we got from the database.
                    .map(|t| r.get(&t.id).cloned().unwrap_or(t))
                    .collect()
            })
            .await?;

        let mut results: Vec<Trigger> =
            Vec::with_capacity(dead.len() + alive_triggers.len());

        // merge live and dead
        results.extend(dead);
        results.extend(alive_triggers);
        // reverse sort the list by trigger id (newer first)
        results.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(PaginatedResponse::from(results, pagination))
    }

    #[tracing::instrument(skip_all, fields(trigger_name = %name, project_id = %context.project_id))]
    pub async fn run_trigger(
        &self,
        context: RequestContext,
        name: String,
        mode: DispatchMode,
    ) -> Result<Run, TriggerError> {
        let trigger = self.get_trigger(context.clone(), name).await?;

        if trigger.status == Status::Cancelled {
            return Err(TriggerError::InvalidStatus(
                "run".to_string(),
                trigger.status,
            ));
        }
        let run =
            dispatch(context, trigger, self.dispatcher_clients.clone(), mode)
                .await?;
        Ok(run)
    }

    #[tracing::instrument(skip_all, fields(trigger_name = %name, project_id = %context.project_id))]
    pub async fn pause_trigger(
        &self,
        context: RequestContext,
        name: String,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        let trigger_id =
            self.get_trigger_id(&context.project_id, &name).await?;

        let status =
            self.get_trigger_status(&context.project_id, &name).await?;
        // if value, check that it's alive.
        if !status.alive() {
            return Err(TriggerError::InvalidStatus(
                Status::Paused.as_operation(),
                status,
            ));
        }
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.pause(&trigger_id)
                .map(|_| w.get(&trigger_id).unwrap())
                .cloned()
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_name = %name, project_id = %context.project_id))]
    pub async fn resume_trigger(
        &self,
        context: RequestContext,
        name: String,
    ) -> Result<Trigger, TriggerError> {
        let triggers = self.triggers.clone();
        let trigger_id =
            self.get_trigger_id(&context.project_id, &name).await?;
        let status =
            self.get_trigger_status(&context.project_id, &name).await?;
        // if value, check that it's alive.
        if !status.alive() {
            return Err(TriggerError::InvalidStatus(
                Status::Scheduled.as_operation(),
                status,
            ));
        }
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.resume(&trigger_id)
                .map(|_| w.get(&trigger_id).unwrap())
                .cloned()
        })
        .await?
    }

    #[tracing::instrument(skip_all, fields(trigger_name = %name, project_id = %context.project_id))]
    pub async fn cancel_trigger(
        &self,
        context: RequestContext,
        name: String,
    ) -> Result<Trigger, TriggerError> {
        let trigger_id =
            self.get_trigger_id(&context.project_id, &name).await?;
        let triggers = self.triggers.clone();
        let status =
            self.get_trigger_status(&context.project_id, &name).await?;
        // if value, check that it's alive.
        if !status.cancelleable() {
            return Err(TriggerError::InvalidStatus(
                Status::Cancelled.as_operation(),
                status,
            ));
        }

        if status == Status::OnDemand {
            let mut trigger =
                self.get_trigger(context.clone(), name.to_string()).await?;
            trigger.status = Status::Cancelled;
            self.store.update_trigger(trigger.clone()).await?;
            e!(
                context = context,
                TriggerStatusUpdated {
                    meta: trigger.meta().into(),
                    old_status: Status::OnDemand.into(),
                    new_status: trigger.status.clone().into(),
                }
            );
            Ok(trigger)
        } else {
            tokio::task::spawn_blocking(move || {
                let mut w = triggers.write().unwrap();
                // We blindly cancel here since we have checked earlier that
                // this trigger is owned by the right project.
                // Otherwise, we won't reach this line.
                w.cancel(&trigger_id)
                    .map(|_| w.get(&trigger_id).unwrap())
                    .cloned()
            })
            .await?
        }
    }

    #[tracing::instrument(skip_all, fields(trigger_name = %name, project_id = %context.project_id))]
    pub async fn delete_trigger(
        &self,
        context: RequestContext,
        name: String,
    ) -> Result<(), TriggerError> {
        let trigger_id =
            self.get_trigger_id(&context.project_id, &name).await?;
        let triggers = self.triggers.clone();

        let meta = Some(TriggerMeta {
            trigger_id: Some(trigger_id.clone().into()),
            name: name.clone(),
        });

        tokio::task::spawn_blocking({
            let trigger_id = trigger_id.clone();
            move || {
                let mut w = triggers.write().unwrap();
                w.remove(&trigger_id)
            }
        })
        .await?;

        self.store
            .delete_trigger(&context.project_id, &trigger_id)
            .await?;
        self.remove_name_from_cache(&name);
        e!(context = context, TriggerDeleted { meta });
        info!("Trigger '{name}' ({trigger_id}) has been deleted!");
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(project_id = %context.project_id))]
    pub async fn delete_project_triggers(
        &self,
        context: RequestContext,
    ) -> Result<(), TriggerError> {
        let project_id = context.project_id.clone();
        let triggers = self.triggers.clone();

        // NOTE: This will stall the spinner if the list is too big.
        tokio::task::spawn_blocking({
            let project_id = project_id.clone();
            move || {
                let mut w = triggers.write().unwrap();
                w.remove_by_project(&project_id)
            }
        })
        .await?;

        // NOTE: We only delete from the database after clearing up the active
        // trigger map to  ensure that we don't lose the race against
        // `perform_checkpoint` which might write in-flight changes before we
        // get a chance to hold the write lock.
        self.store.delete_triggers_by_project(&project_id).await?;
        // TODO (#6): Remove all names for that particular project.
        //self.remove_name_from_cache(&name);
        info!("All triggers for project {project_id} has been deleted!");
        Ok(())
    }

    pub async fn shutdown(&self) {
        {
            let mut spinner = self.spinner.lock().unwrap();
            // will drop the current spinner after shutdown.
            if let Some(spinner) = spinner.take() {
                spinner.shutdown();
            } else {
                info!("SpinnerController has already been shutdown!");
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
        name: &str,
    ) -> Result<Status, TriggerError> {
        let status = self.store.get_status(project, name).await?;
        // if None -> Trigger doesn't exist
        status.ok_or_else(|| TriggerError::NotFound(name.to_owned()))
    }
}
