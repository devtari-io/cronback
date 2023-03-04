use std::sync::{Arc, Mutex, RwLock};

use tracing::info;

use super::{
    spinner::{Spinner, SpinnerHandle},
    triggers::{ActiveTriggerMap, TriggerError},
};
use proto::trigger_proto::Trigger;
use shared::service::ServiceContext;

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

    pub fn start(&self) {
        let mut spinner = self.spinner.lock().unwrap();
        if spinner.is_some() {
            info!("EventScheduler has already started!");
            return;
        }
        // TODO: Load state from database
        *spinner = Some(
            Spinner::new(self.context.clone(), self.triggers.clone()).start(),
        );
    }

    pub async fn install_trigger(
        &self,
        trigger: Trigger,
    ) -> Result<(), TriggerError> {
        let triggers = self.triggers.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = triggers.write().unwrap();
            w.add_or_update(trigger)
        })
        .await?
    }

    pub async fn get_trigger(
        &self,
        id: String,
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
