use std::{
    collections::BinaryHeap,
    sync::Arc,
    sync::RwLock,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use tokio::runtime::Handle;
use tracing::info;

use shared::service::ServiceContext;

use super::triggers::{ActiveTriggerMap, TemporalState};

pub(crate) struct Spinner {
    tokio_handle: Handle,
    triggers: Arc<RwLock<ActiveTriggerMap>>,
    shutdown: Arc<RwLock<bool>>,
    context: ServiceContext,
    //temporal_states: BinaryHeap<TemporalState>,
}

pub(crate) struct SpinnerHandle {
    join: JoinHandle<()>,
    shutdown_signal: Arc<RwLock<bool>>,
}

impl SpinnerHandle {
    pub fn shutdown(self) {
        info!("Attempting to shutdown spinner");
        {
            let mut guard = self.shutdown_signal.write().unwrap();
            *guard = true;
        }
        info!("Waiting for spinner to shutdown");
        let _ = self.join.join();
        info!("Spinner terminated!");
    }
}

/*
 * Design thoughts:
 * - Spinner is not responsible for retries. The async dispatch logic
 *   will need to take care of this if the RPC call failed.
 * - The spinner will only consider triggers already installed in TriggerMap.
 * - Short sweet locking of TriggerMap to reduce contention. Write-locking TriggerMap
 *   for too long will cause grpc handlers to block.
 * - The design needs to handles wall clock time shifts nicely (forwards or
 *   backwards)
 * - Spinner will pop as many triggers from the min-heap as long as the next_tick
 *   is smaller or equal to the current timestamp. The rest is kept for the next
 *   tick.
 * - We don't need to drop the tail of the min-heap (after drain) if no change
 *   happened to triggers map.
 *
 *   ** Shared State & Concurrency Control **
 *   Because we want to reduce lock contention and freezing the spinner as much
 *   as possible, the design calls for the following requirements:
 *      - We maintain a shared Trigger Map behind a read-write lock that can be
 *        accessed from the EventScheduler API, SchedulerAPIHandler, and more.
 *      - The EventScheduler owns the spinner exclusively. The spinner is self
 *        contained and has a well defined job.
 *      - Spinner runs on its own dedicated thread and delegates IO work (dispatch)
 *        via `tokio_handle`.
 *      - EventScheduler is the only entry point for adding/removing/querying
 *       information about triggers. The EventScheduler will performing database
 *       writes to persist changes, but for the purposes of this component, our
 *       in-memory state (TriggerMap) is the source-of-truth.
 *      - The Spinner maintains the `TemporalState` locally. Each installed trigger
 *        will have a corresponding TemporalState that tells the spinner the time
 *        point to trigger the next event. We keep TemporalState(s) in a min-heap
 *        sorted by the next tick for installed triggers. This state is created
 *        from the TriggerMap and is checked/updated on each iteration of the loop.
 *      - EventScheduler will only edit the trigger map as it receives API calls.
 *      - TriggerMap maintains a set a dirty trigger Ids to let the Spinner know
 *        which triggers require TemporalState rebuilding.
 *
 */
impl Spinner {
    pub fn new(
        context: ServiceContext,
        triggers: Arc<RwLock<ActiveTriggerMap>>,
    ) -> Self {
        Self {
            tokio_handle: Handle::current(),
            shutdown: Arc::new(RwLock::new(false)),
            context,
            triggers,
            //BinaryHeap,
        }
    }
    pub fn start(self) -> SpinnerHandle {
        let shutdown_signal = self.shutdown.clone();
        let join = std::thread::Builder::new()
            .name("SPINNER".to_owned())
            .spawn(|| {
                self.run_forever();
            })
            .expect("Spinner thread failed to start!");

        SpinnerHandle {
            join,
            shutdown_signal,
        }
    }

    #[tracing::instrument(skip_all)]
    fn run_forever(mut self) {
        let config = self.context.load_config();
        let yield_max_duration =
            Duration::from_millis(config.scheduler.spinner_yield_max_ms);
        'tick_loop: loop {
            {
                let shutdown = self.shutdown.read().unwrap();
                if *shutdown {
                    break 'tick_loop;
                }
            }
            {
                // Is the state dirty?
                if !self.triggers.read().unwrap().is_dirty() {
                    info!("Triggers updated, reloading...");
                    // TODO reload the triggers that has been updated. Or
                    // re-construct the entire temporal state.
                    let mut w = self.triggers.write().unwrap();
                    w.reset_dirty();
                }
            }
            /*
             * The rough plan:
             *
             * 1. Go over all installed triggers that have next_tick <= now()
             * Those are removed from the min-heap. Keep the list of removed
             * triggers until we finish up the loop.
             *
             * 2. Dispatch a new event (async) for each of those triggers.
             */
            let instant = Instant::now();
            info!("[REMOVE ME] Tick...");
            self.tokio_handle.spawn(async move {
                info!("async-dispatch test");
            });
            /*
             * 3. Read-Lock TriggerMap
             * 4. Check if we have dirty triggers, fetch their contents.
             * 5. Calculate temporal state for the removed triggers and re-insert.
             * 6. Update existing temporal state if triggers have been updated.
             * 7. loop
             */

            // This number indicates how busy the machine is. The closer to zero
            // the busier we are. That said, it's not an accurate indicator of
            // the overall system performance as we might be overwhelming the
            // tokio runtime by the number of the async dispatches we are creating.
            //
            // For those, the latency is measured separately.
            let remaining =
                yield_max_duration.saturating_sub(instant.elapsed());
            if remaining != Duration::ZERO {
                // TODO: Consider using spin_sleep
                std::thread::sleep(remaining);
            }
        }
    }

    fn rebuild_temporal_state(&mut self) {
        // for all active triggers, determine the next tick.
        //self.temporal_states.clear();
        let mut triggers = self.triggers.read().unwrap();
        for trigger in triggers.triggers_iter() {
            let trigger_id = trigger.get().id.clone();
        }
    }
}
