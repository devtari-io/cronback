use std::sync::{Arc, Mutex, RwLock};

use tracing::info;

use super::{
    spinner::{Spinner, SpinnerHandle},
    triggers::TriggerMap,
};
use shared::service::ServiceContext;

/**
 *
 * EventScheduler is the gateway to the scheduling and dispatch thread (spinner)
 * It's designed to be easily shared with inner mutability and minimal locking
 * to reduce contention.
 *
 * EventScheduler owns
 *   - Active TriggerMap
 *   - The SpinnerHandle
 * Provides:
 *   - API to install/remove/update/query triggers
 *   - Start/pause the spinner.
 *
 * Needs to take care of:
 *   - Compaction/eviction: remove expired triggers
 */
pub(crate) struct EventScheduler {
    context: ServiceContext,
    triggers: Arc<RwLock<TriggerMap>>,
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

    pub fn install_trigger(&self) {
        self.trigger_updated("trig_somethingsomething".to_owned())
    }

    pub fn shutdown(&self) {
        let mut spinner = self.spinner.lock().unwrap();
        // will drop the current spinner after shutdown.
        if let Some(spinner) = spinner.take() {
            spinner.shutdown();
            let mut triggers = self.triggers.write().unwrap();
            triggers.dirty_triggers.clear();
            triggers.state.clear();
        } else {
            info!("EventScheduler has already been shutdown!");
        }
        // TODO: Do we need to flush anything to the database/filesystem?
    }

    //// PRIVATE
    fn trigger_updated(&self, trigger_id: String) {
        let mut w = self.triggers.write().unwrap();
        w.dirty_triggers.insert(trigger_id);
    }
}
