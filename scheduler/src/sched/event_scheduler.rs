use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use chrono_tz::UTC;
use tracing::info;

use super::{
    spinner::{Spinner, SpinnerHandle},
    triggers::{ActiveTriggerMap, TriggerError},
};
use proto::scheduler_proto::InstallTrigger;
use shared::{
    grpc_client_provider::DispatcherClientProvider,
    service::ServiceContext,
    types::{OwnerId, Status, Trigger, TriggerId},
};

/**
 *
 * EventScheduler is the gateway to the scheduling and dispatch thread (spinner)
 * It's designed to be easily shared with inner mutability and minimal locking
 * to reduce contention.
 *
 *
 * Event Scheduler also wraps the database. I'll load and store triggers from
 * the database as needed. Installing a new trigger happens on two steps:
 * - Inserting the trigger in the database
 * - Adding the trigger to the ActiveTriggerMap (while holding a write lock)
 *
 * This is designed like this to minimise locking the ActiveTriggerMap (vs. making
 * database queries from the TriggerMap while holding the write lock unnecessarily)
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
 *   - Compaction/eviction: remove expired triggers
 */
pub(crate) struct EventScheduler {
    context: ServiceContext,
    triggers: Arc<RwLock<ActiveTriggerMap>>,
    spinner: Mutex<Option<SpinnerHandle>>,
}

impl EventScheduler {
    pub fn new(context: ServiceContext) -> Self {
        Self {
            context,
            triggers: Arc::default(),
            spinner: Mutex::default(),
        }
    }

    pub fn start(
        &self,
        dispatcher_client_provider: Arc<DispatcherClientProvider>,
    ) {
        let mut spinner = self.spinner.lock().unwrap();
        if spinner.is_some() {
            info!("EventScheduler has already started!");
            return;
        }
        // TODO: Load state from database
        *spinner = Some(
            Spinner::new(
                self.context.clone(),
                self.triggers.clone(),
                dispatcher_client_provider,
            )
            .start(),
        );
    }

    pub async fn install_trigger(
        &self,
        install_trigger: InstallTrigger,
    ) -> Result<Trigger, TriggerError> {
        let id = TriggerId::new(&OwnerId(install_trigger.owner_id.clone()));

        let trigger = Trigger {
            id,
            owner_id: install_trigger.owner_id.into(),
            reference_id: install_trigger.reference_id,
            name: install_trigger.name,
            description: install_trigger.description,
            created_at: Utc::now().with_timezone(&UTC),
            emit: install_trigger.emit.into_iter().map(|e| e.into()).collect(),
            payload: install_trigger.payload.unwrap().into(),
            schedule: install_trigger.schedule.map(|s| s.into()),
            status: Status::Active,
        };

        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.add_or_update(trigger)
        })
        .await?
    }

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
    }

    pub fn shutdown(&self) {
        let mut spinner = self.spinner.lock().unwrap();
        // will drop the current spinner after shutdown.
        if let Some(spinner) = spinner.take() {
            spinner.shutdown();
            let mut triggers = self.triggers.write().unwrap();
            triggers.clear();
        } else {
            info!("EventScheduler has already been shutdown!");
        }
        // TODO: Do we need to flush anything to the database/filesystem?
    }
}
