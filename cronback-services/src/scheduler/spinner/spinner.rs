use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::vec;

use chrono::{DateTime, Utc};
use lib::clients::ScopedDispatcherSvcClient;
use lib::prelude::*;
use lib::service::ServiceContext;
use lib::GrpcClientProvider;
use metrics::{counter, gauge, histogram};
use tokio::runtime::Handle;
use tracing::{debug, info, trace, warn, Instrument};

use super::active_triggers::{ActiveTriggerMap, TriggerTemporalState};
use super::dispatch::{self, DispatchError, DispatchMode};
use crate::scheduler::db_model::triggers::Status;
use crate::scheduler::SchedulerService;

pub(crate) struct Spinner {
    tokio_handle: Handle,
    triggers: Arc<RwLock<ActiveTriggerMap>>,
    shutdown: Arc<RwLock<bool>>,
    context: ServiceContext<SchedulerService>,
    dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherSvcClient>>,
}

pub(crate) struct SpinnerHandle {
    join: JoinHandle<()>,
    shutdown_signal: Arc<RwLock<bool>>,
}

struct InflightDispatch {
    pub trigger_id: TriggerId,
    pub ran_at: DateTime<Utc>,
    pub handle: tokio::task::JoinHandle<Result<(), DispatchError>>,
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

///  **Design thoughts:**
///   - [Spinner] is not responsible for dispatch retries. The async dispatch
///     logic will need to take care of this if the RPC call failed.
///   - The spinner will only consider triggers already installed in
///     [ActiveTriggerMap].
///   - Short sweet locking of [ActiveTriggerMap] to reduce contention.
///     Write-locking [ActiveTriggerMap] for too long will cause grpc handlers
///     to block.
///   - The design needs to handles wall clock time shifts nicely (forwards or
///     backwards)
///   - Spinner will pop as many triggers from the min-heap as long as the
///     next_tick is smaller or equal to the current timestamp. The rest is kept
///     for the next tick.
///   - We don't need to drop the tail of the min-heap (after drain) if no
///     change happened to triggers map.
///
///  **Shared State & Concurrency Control**
///   Because we want to reduce lock contention and freezing the spinner as
///   much as possible, the design calls for the following requirements:
///   - We maintain a shared Trigger Map behind a read-write lock that can be
///     accessed from the [SpinnerController] API, [SchedulerAPIHandler], and
///     more.
///   - The [SpinnerController] owns the spinner exclusively. The spinner is
///     self contained and has a well defined job.
///   - Spinner runs on its own dedicated thread and delegates IO work
///     (dispatch) via `tokio_handle`.
///   - [SpinnerController] is the only entry point for adding/removing/querying
///     information about triggers. The [SpinnerController] will performing
///     database writes to persist changes, but for the purposes of this
///     component, our in-memory state ([ActiveTriggerMap]) is the
///     source-of-truth.
///   - The [Spinner] maintains the [TriggerTemporalState] locally. Each
///     installed trigger will have a corresponding TemporalState that tells the
///     spinner the time point to trigger the next event. We keep
///     TemporalState(s) in a min-heap sorted by the next tick for installed
///     triggers. This state is created from the [ActiveTriggerMap] and is
///     checked/updated on each iteration of the loop.
///   - [SpinnerController] will only edit the trigger map as it receives API
///     calls.
///   - The [ActiveTriggerMap] maintains a set a dirty trigger Ids to let the
///     Spinner know which triggers require [TriggerTemporalState] rebuilding.
///
///     [SpinnerController]: super::controller::SpinnerController
///     [SchedulerAPIHandler]: crate::handler::SchedulerAPIHandler
impl Spinner {
    pub fn new(
        context: ServiceContext<SchedulerService>,
        triggers: Arc<RwLock<ActiveTriggerMap>>,
        dispatcher_clients: Arc<GrpcClientProvider<ScopedDispatcherSvcClient>>,
    ) -> Self {
        Self {
            tokio_handle: Handle::current(),
            shutdown: Arc::new(RwLock::new(false)),
            context,
            triggers,
            dispatcher_clients,
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
    fn run_forever(self) {
        let mut temporal_states: BinaryHeap<Reverse<TriggerTemporalState>> =
            Default::default();
        let mut inflight_dispatches: Vec<InflightDispatch> = Vec::new();
        let config = self.context.service_config();
        let yield_max_duration =
            Duration::from_millis(config.spinner_yield_max_ms);
        let max_triggers_per_tick = config.max_triggers_per_tick;
        'tick_loop: loop {
            {
                let shutdown = self.shutdown.read().unwrap();
                if *shutdown {
                    break 'tick_loop;
                }
            }

            // Successful dispatches should update the last_ran_at in
            // ActiveTriggerMap and compacted.
            {
                // Scoped to drop memory asap.
                let mut pending_dispatches =
                    Vec::with_capacity(inflight_dispatches.len());
                let mut success_dispatches =
                    Vec::with_capacity(inflight_dispatches.len());
                for inflight in inflight_dispatches.drain(..) {
                    if inflight.handle.is_finished() {
                        // Success? This is quick since it's already finished.
                        if self
                            .tokio_handle
                            .block_on(inflight.handle)
                            .unwrap()
                            .is_ok()
                        {
                            success_dispatches
                                .push((inflight.trigger_id, inflight.ran_at));
                        }
                    } else {
                        // keep it around, we are still waiting for them.
                        pending_dispatches.push(inflight);
                    }
                }
                // continue tracking those who didn't finish yet.
                inflight_dispatches = pending_dispatches;
                {
                    // Those who succeeded should be updated in the active map
                    let mut w = self.triggers.write().unwrap();
                    for (trigger_id, ran_at) in success_dispatches {
                        w.update_last_ran_at(&trigger_id, ran_at);
                    }
                }
            }
            counter!(
                "inflight_dispatches_total",
                inflight_dispatches.len() as u64
            );

            /*
             * 1. Go over all installed triggers that have next_tick <= now()
             * Those are removed from the min-heap. Keep the list of removed
             * triggers until we finish up the loop.
             *
             */
            let mut dispatch_queue = vec![];
            for _ in 0..max_triggers_per_tick {
                let Some(temporal_state) = temporal_states.peek() else {
                    break;
                };
                if temporal_state.0.next_tick <= Utc::now() {
                    let temporal_state = temporal_states.pop().unwrap().0;
                    trace!(
                        "Adding trigger {} to the dispatch queue",
                        temporal_state.trigger_id,
                    );
                    dispatch_queue.push(temporal_state);
                } else {
                    // The rest is in the future.
                    break;
                }
            }
            if dispatch_queue.len() == max_triggers_per_tick as usize {
                warn!(
                    "Reached max dispatches per tick ({}), some triggers will \
                     be deferred",
                    max_triggers_per_tick
                );
            }

            /*
             * 2. Dispatch a new event (async) for each of those triggers.
             */
            let instant = Instant::now();
            trace!(
                "[TICK] temporal_state {} and dispatch_queue {}",
                temporal_states.len(),
                dispatch_queue.len(),
            );
            for trigger in dispatch_queue.iter() {
                let id = trigger.trigger_id.clone();
                let scheduled_time = trigger.next_tick;
                let lag = Utc::now()
                    .signed_duration_since(scheduled_time)
                    .num_milliseconds() as f64;
                histogram!("spinner.dispatch_lag_seconds", lag / 1000.0);
                if lag > 10_000.0 {
                    warn!(
                        "Spinner lag has exceeded 10s, you might need to \
                         increase `max_triggers_per_tick` (current \
                         '{max_triggers_per_tick}') or reduce \
                         `spinner_yield_max_ms` (current '{}')",
                        config.spinner_yield_max_ms
                    );
                }

                if let Some(handle) = self.dispatch(&id) {
                    inflight_dispatches.push(InflightDispatch {
                        trigger_id: id.clone(),
                        ran_at: Utc::now(),
                        handle,
                    });
                }
            }
            /*
             * 3. Write-Lock ActiveTriggerMap
             * 4. Calculate temporal state for the removed triggers and
             *    re-insert.
             */
            if !dispatch_queue.is_empty() {
                // Maybe individual triggers need to be advanced.
                let mut w = self.triggers.write().unwrap();
                for mut trigger in dispatch_queue {
                    // Those states that yield None in advance will be dropped.
                    if let Some(next_tick) = w.advance(&trigger.trigger_id) {
                        trace!(
                            "Trigger {} next trigger time is {}",
                            trigger.trigger_id,
                            next_tick
                        );
                        // _can_ be avoided if the trigger map is dirty, but
                        // will keep it for simplicity.
                        trigger.next_tick = next_tick;
                        temporal_states.push(Reverse(trigger));
                    } else {
                        // This trigger is no longer active. We should flush and
                        // compact.
                        w.add_to_awaiting_db_flush(trigger.trigger_id);
                    }
                }
            }
            /*
             * 5. Check if we have dirty triggers, fetch their contents.
             * 6. Rebuild temporal state if needed.
             */
            // Is the state dirty?
            if self.triggers.read().unwrap().is_dirty() {
                trace!("Triggers updated, reloading...");
                // TODO only reload the triggers that has been updated. Or
                // re-construct the entire temporal state.
                let mut w = self.triggers.write().unwrap();
                temporal_states = w.build_temporal_state();
                gauge!(
                    "spinner.active_triggers_total",
                    temporal_states.len() as f64
                );
            }

            // This indicates how busy the machine is. The closer to zero
            // the busier we are. That said, it's not an accurate indicator of
            // the overall system performance as we might be overwhelming the
            // tokio runtime by the number of the async dispatches we are
            // creating.
            //
            // For those, the latency is measured separately.
            let remaining =
                yield_max_duration.saturating_sub(instant.elapsed());
            histogram!(
                "spinner.yield_duration_ms",
                remaining.as_millis() as f64,
            );
            if remaining != Duration::ZERO {
                std::thread::sleep(remaining);
            }
        }
    }

    fn dispatch(
        &self,
        trigger_id: &TriggerId,
    ) -> Option<tokio::task::JoinHandle<Result<(), DispatchError>>> {
        let trigger = {
            let r = self.triggers.read().unwrap();
            let Some(trigger) = r.get(trigger_id) else {
                // The trigger could have been removed from the active trigger
                // maps, let's just ignore it.
                return None;
            };
            trigger.clone()
        };

        if trigger.status == Status::Paused {
            let handle = self.tokio_handle.spawn(
                async move {
                    // We probably should persist this fake run somewhere.
                    debug!(
                        "Skipping dispatch of PAUSED trigger {}",
                        trigger.id
                    );
                    Ok(())
                }
                .instrument(tracing::Span::current()),
            );
            return Some(handle);
        }
        // TODO:
        // Can we dispatch? if we can't, we should retry a few times, then we
        // drop into  failsafe mode.
        // In failsafe mode:
        //  - Stop the if spinner
        //  - Run a health check loop to find alive dispatchers.
        //  - Once a live dispatcher is found, re-init the spinner and continue.
        let provider = self.dispatcher_clients.clone();
        // TODO: Think about when should we persist the fact that we dispatched.
        let handle = self.tokio_handle.spawn(
            async move {
                // TODO add a few retries if not logic error.
                // We are throwing away the returned Run object to save
                // memory we are only interested in knowing if
                // we errored or not.
                dispatch::dispatch(
                    RequestContext::new(
                        // We generate a new request Id as this is a
                        // system-generated request.
                        RequestId::new(),
                        trigger.project_id.clone(),
                    ),
                    trigger,
                    provider,
                    DispatchMode::Async,
                )
                .await
                .map(|_| ())
            }
            .instrument(tracing::Span::current()),
        );
        Some(handle)
    }
}
