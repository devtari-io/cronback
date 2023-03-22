use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::iter::Peekable;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::{Tz, UTC};
use cron::{OwnedScheduleIterator, Schedule as CronSchedule};
use shared::database::trigger_store::TriggerStoreError;
use shared::types::{Schedule, Status, Trigger, TriggerId};
use thiserror::Error;
use tracing::info;

use super::event_dispatcher::DispatchError;

#[derive(Error, Debug)]
pub(crate) enum TriggerError {
    #[error("Cannot parse cron expression")]
    CronParseError(#[from] cron::error::Error),
    #[error(
        "Unrecognized timezone '{0}' was supplied, are you sure this is an \
         IANA timezone?"
    )]
    InvalidTimezone(String),
    #[error("Trigger '{0}' should not have passed validation!")]
    MalformedTrigger(TriggerId),
    #[error("Trigger with Id '{0}' is unknown to this scheduler!")]
    NotFound(TriggerId),
    #[error("Cannot perform operation on a trigger with status '{0}'")]
    InvalidStatus(Status),
    //join error
    #[error("Internal async processing failure!")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Operation on underlying trigger store failed: {0}")]
    TriggerStoreError(#[from] TriggerStoreError),
    #[error("Cannot dispatch an invocation for this trigger")]
    InvocationError(#[from] DispatchError),
}

///
/// Maintains the set of `active` triggers in memory. Expired triggers are
/// evicted to save space.
///
/// dirty_triggers holds the set of triggers that has been updated since the
/// last time the spinner has looked at it. The spinner resets the set after
/// reloading.
#[derive(Default)]
pub(crate) struct ActiveTriggerMap {
    state: HashMap<TriggerId, ActiveTrigger>,
    /// The set of trigger Ids that has been updated
    dirty: bool,
    awaiting_db_flush: HashSet<TriggerId>,
}

impl ActiveTriggerMap {
    /// Inserts or updates a trigger if exists
    pub fn add_or_update(
        &mut self,
        trigger: Trigger,
        fast_forward: bool,
    ) -> Result<Trigger, TriggerError> {
        // TODO: We should instead convert active_trigger back to Trigger to
        // get the updated status
        let cloned_trigger = trigger.clone();
        let trigger_id = trigger.id.clone();
        let active_trigger = ActiveTrigger::try_from(trigger, fast_forward)?;
        self.state.insert(trigger_id, active_trigger);
        self.mark_dirty();
        Ok(cloned_trigger)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn awaiting_db_flush(&self) -> HashSet<TriggerId> {
        self.awaiting_db_flush.clone()
    }

    pub fn clear_db_flush(&mut self) {
        self.awaiting_db_flush.clear();
    }

    pub fn add_to_awaiting_db_flush(&mut self, trigger_id: TriggerId) {
        // Mark this trigger as dirty so that we can persist it
        self.awaiting_db_flush.insert(trigger_id);
    }

    pub fn is_trigger_retired(&self, trigger_id: &TriggerId) -> bool {
        self.state
            .get(trigger_id)
            .map(|t| {
                [Status::Canceled, Status::Expired].contains(&t.get().status)
            })
            .unwrap_or(false)
    }

    // Can be used to remove an expired/cancelled trigger after flush
    pub fn pop_trigger(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Option<ActiveTrigger> {
        let res = self.state.remove(trigger_id);
        if res.is_some() {
            // not strictly necessary, but without it the spinner will report
            // the wrong number of active triggers to metrics.
            self.mark_dirty();
        }
        res
    }

    // Should be used to only push dead/retired triggers.
    pub fn push_trigger(
        &mut self,
        trigger_id: TriggerId,
        trigger: ActiveTrigger,
    ) {
        self.state.insert(trigger_id, trigger);
        // not strictly necessary, but without it the spinner will report
        // the wrong number of active triggers to metrics.
        self.mark_dirty();
    }

    pub fn clear(&mut self) {
        self.state.clear();
        self.reset_dirty();
    }

    /// Scans the set of active triggers and creates temporal states without
    /// advancing any triggers.
    pub fn build_temporal_state(
        &mut self,
    ) -> BinaryHeap<Reverse<TriggerTemporalState>> {
        let mut new_state = BinaryHeap::new();
        for trigger in self.state.values_mut() {
            if let Some(tick) = trigger.peek() {
                let state = TriggerTemporalState {
                    trigger_id: trigger.get().id.clone(),
                    next_tick: tick,
                };
                info!(
                    "Trigger '{}' next trigger point is {}",
                    state.trigger_id, tick
                );
                new_state.push(Reverse(state));
            }
        }
        self.reset_dirty();
        new_state
    }

    pub fn get(&self, id: &TriggerId) -> Option<&Trigger> {
        self.state.get(id).map(|t| t.get())
    }

    /*
     * Advance ensures that the next tick is not the same as current tick.
     * I hear you asking, why do we need that?
     *
     * We need this because:
     * - We don't want to advance everything when we build a new temporal
     *   states, as this will incorrectly advance triggers that are still
     *   due.
     * - When temporal state is created, we only peek(), this makes the first
     *   iteration a bit awkward, when we advance after executing the
     *   trigger, advance() will return the same time point because we have
     *   never next()ed it. This _hack_ ensures that we will fast-forward in
     *   this rare case.
     * - This also ensures that, for any reason, we skip duplicates in the
     *   run_at list if we didn't catch this in validation.
     */
    pub fn advance(&mut self, trigger_id: &TriggerId) -> Option<DateTime<Tz>> {
        let Some(trigger) = self.state.get_mut(trigger_id) else {
            return None;
        };

        if !trigger.is_alive() {
            return None;
        }

        let next_tick = trigger.advance();

        if next_tick.is_some() {
            return next_tick;
        }

        // We should not be active anymore.
        self.update_status(trigger_id, Status::Expired, &[])
            .unwrap();
        None
    }

    pub fn pause(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Result<(), TriggerError> {
        self.update_status(
            trigger_id,
            Status::Paused,
            &[Status::Canceled, Status::Expired],
        )
    }

    pub fn resume(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Result<(), TriggerError> {
        self.update_status(
            trigger_id,
            Status::Active,
            &[Status::Canceled, Status::Expired],
        )
    }

    pub fn cancel(
        &mut self,
        trigger_id: &TriggerId,
    ) -> Result<(), TriggerError> {
        self.update_status(trigger_id, Status::Canceled, &[])
    }

    fn update_status(
        &mut self,
        trigger_id: &TriggerId,
        new_status: Status,
        reject_statuses: &[Status],
    ) -> Result<(), TriggerError> {
        let Some(trigger) = self.state.get_mut(trigger_id) else {
            return Err(TriggerError::NotFound(trigger_id.clone()));
        };

        if reject_statuses.contains(&trigger.get().status) {
            return Err(TriggerError::InvalidStatus(
                trigger.get().status.clone(),
            ));
        }

        if trigger.update_status(new_status) {
            self.add_to_awaiting_db_flush(trigger_id.clone());
        }
        Ok(())
    }

    pub fn update_last_invoked_at(
        &mut self,
        trigger_id: &TriggerId,
        invoked_at: DateTime<Utc>,
    ) {
        let Some(trigger) = self.state.get_mut(trigger_id) else {
             return;
        };
        // Keep the last known invocation time (if set) unless we are seeing
        // a more recent one.
        let new_val = match trigger.last_invoked_at() {
            // If we have never seen an invocation time, set it
            | None => Some(invoked_at),
            // If we are seeing a more recent invocation time, update it
            | Some(last_known) if invoked_at > last_known => Some(invoked_at),
            // Use the last known invocation time
            | Some(last_known) => Some(last_known),
        };

        // We are guarding this because we should not add this trigger to
        // `awaiting_db_flush` if we didn't really update it.
        if trigger.inner.hidden_last_invoked_at != new_val {
            trigger.inner.hidden_last_invoked_at = new_val;
            self.add_to_awaiting_db_flush(trigger_id.clone());
        }
    }

    //// PRIVATE
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn reset_dirty(&mut self) {
        self.dirty = false;
    }
}

/// Metadata exclusively owned by the spinner, keeps the Id of the installed
/// trigger along with its next tick.
///
/// The spinner maintains a max-heap of those jobs to determine which Triggers
/// need to be evaluated at each loop.
pub(crate) struct TriggerTemporalState {
    #[allow(unused)]
    pub trigger_id: TriggerId,
    // time of the next tick
    pub next_tick: DateTime<Tz>,
}

impl PartialOrd for TriggerTemporalState {
    fn partial_cmp(
        &self,
        other: &TriggerTemporalState,
    ) -> Option<std::cmp::Ordering> {
        self.next_tick.partial_cmp(&other.next_tick)
    }
}
impl Ord for TriggerTemporalState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next_tick.cmp(&other.next_tick)
    }
}

impl PartialEq for TriggerTemporalState {
    fn eq(&self, other: &Self) -> bool {
        self.next_tick == other.next_tick
    }
}
impl Eq for TriggerTemporalState {}

/// A type that abstracts the generator for future ticks, whether it's backed
/// by a cron patter, or a list of time points.
pub(crate) enum TriggerFutureTicks {
    CronPattern {
        next_ticks: Peekable<OwnedScheduleIterator<Tz>>,
        remaining: Option<u64>,
    },
    RunAt {
        run_at: BinaryHeap<Reverse<DateTime<Tz>>>,
        remaining: u64,
    },
}

impl TriggerFutureTicks {
    pub fn from_schedule(
        schedule_raw: &Schedule,
        last_invoked_at: Option<DateTime<Utc>>,
    ) -> Result<Self, TriggerError> {
        match schedule_raw {
            | Schedule::Recurring(cron) => {
                let raw_pattern = cron.cron.clone().unwrap();
                let cron_schedule = CronSchedule::from_str(&raw_pattern)?;
                let tz: Tz = cron.timezone.parse().map_err(|_| {
                    TriggerError::InvalidTimezone(cron.timezone.clone())
                })?;
                let next_ticks = if let Some(last_invoked_at) = last_invoked_at
                {
                    cron_schedule
                        .after_owned(last_invoked_at.with_timezone(&UTC))
                        .peekable()
                } else {
                    cron_schedule.upcoming_owned(tz).peekable()
                };
                let remaining = if cron.limit > 0 {
                    Some(cron.remaining)
                } else {
                    None
                };
                Ok(TriggerFutureTicks::CronPattern {
                    next_ticks,
                    remaining,
                })
            }
            | Schedule::RunAt(run_at) => {
                let mut ticks = BinaryHeap::new();
                let last_invoked_at =
                    last_invoked_at.unwrap_or(Utc::now()).with_timezone(&UTC);
                let mut remaining = 0;

                for ts in run_at.timepoints.iter() {
                    if *ts > last_invoked_at {
                        remaining += 1;
                        // Reversed to make this min-heap
                        ticks.push(Reverse(*ts));
                    }
                }
                Ok(TriggerFutureTicks::RunAt {
                    run_at: ticks,
                    remaining,
                })
            }
        }
    }

    // Advances the iterator and peeks the following item
    pub fn advance_and_peek(&mut self) -> Option<DateTime<Tz>> {
        let _ = self.peek_or_next(true);
        self.peek_or_next(false)
    }

    // Looks at the next tick.
    pub fn peek(&mut self) -> Option<DateTime<Tz>> {
        self.peek_or_next(false)
    }

    pub fn remaining(&self) -> Option<u64> {
        match self {
            | Self::CronPattern { remaining, .. } => *remaining,
            | Self::RunAt { remaining, .. } => Some(*remaining),
        }
    }

    fn peek_or_next(&mut self, next: bool) -> Option<DateTime<Tz>> {
        match self {
            // We hit the run limit.
            | Self::CronPattern {
                remaining: Some(events_limit),
                ..
            } if *events_limit == 0 => None,
            // We might have more runs
            | Self::CronPattern {
                next_ticks,
                remaining: remaining_events_limit,
                ..
            } => {
                if next {
                    let n = next_ticks.next();
                    if n.is_some() {
                        if let Some(events_limit) = remaining_events_limit {
                            // consume events.
                            *events_limit -= 1;
                        }
                    }
                    n
                } else {
                    next_ticks.peek().cloned()
                }
            }
            | Self::RunAt { run_at, remaining } => {
                if next {
                    let res = run_at.pop().map(|f| f.0);
                    *remaining -= 1;
                    res
                } else {
                    run_at.peek().map(|f| f.0)
                }
            }
        }
    }
}

// A wrapper around Trigger suitable for scheduler operations.
pub(crate) struct ActiveTrigger {
    inner: Trigger,
    ticks: TriggerFutureTicks,
}

impl ActiveTrigger {
    fn try_from(
        trigger: Trigger,
        fast_forward: bool,
    ) -> Result<Self, TriggerError> {
        // Do we have a cron pattern or a set of time points?
        let k = trigger.schedule.as_ref().ok_or_else(|| {
            TriggerError::MalformedTrigger(trigger.id.clone())
        })?;
        // On fast forward, we ignore the last invocation time.
        let last_invoked_at = if fast_forward {
            None
        } else {
            trigger.hidden_last_invoked_at
        };
        let ticks = TriggerFutureTicks::from_schedule(k, last_invoked_at)?;
        // We assume that Trigger.schedule is never None
        Ok(Self {
            inner: trigger,
            ticks,
        })
    }

    pub fn get(&self) -> &Trigger {
        &self.inner
    }

    pub fn peek(&mut self) -> Option<DateTime<Tz>> {
        self.ticks.peek()
    }

    // Active means that it should continue to live in the spinner map. A paused
    // trigger is considered active, but it won't be invoked. We will advance
    // its clock as if it was invoked though.
    pub fn is_alive(&self) -> bool {
        self.inner.status == Status::Active
            || self.inner.status == Status::Paused
    }

    pub fn advance(&mut self) -> Option<DateTime<Tz>> {
        let res = self.ticks.advance_and_peek();
        let schedule = self.inner.schedule.as_mut().unwrap();
        match schedule {
            | Schedule::Recurring(cron) => {
                cron.remaining = self.ticks.remaining().unwrap_or(0);
            }
            | Schedule::RunAt(run_at) => {
                run_at.remaining = self.ticks.remaining().unwrap_or(0);
            }
        };
        res
    }

    pub fn last_invoked_at(&self) -> Option<DateTime<Utc>> {
        self.inner.hidden_last_invoked_at
    }

    // Returns true if state has changed.
    fn update_status(&mut self, new_status: Status) -> bool {
        if self.inner.status != new_status {
            self.inner.status = new_status;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use shared::timeutil::parse_iso8601;
    use shared::types::{
        Cron,
        OwnerId,
        Payload,
        RunAt,
        Schedule,
        Status,
        Trigger,
        TriggerId,
    };

    use super::*;

    fn create_cron_schedule(pattern: &str, cron_events_limit: u64) -> Schedule {
        Schedule::Recurring(Cron {
            cron: Some(pattern.into()),
            timezone: "Europe/London".into(),
            limit: cron_events_limit,
            remaining: cron_events_limit,
        })
    }

    fn create_run_at(timepoints: Vec<DateTime<Tz>>) -> Schedule {
        let remaining = timepoints.len() as u64;

        Schedule::RunAt(RunAt {
            timepoints,
            remaining,
        })
    }

    fn create_trigger(sched: Schedule) -> Trigger {
        let owner_id = OwnerId("asoli".to_owned());
        let id = TriggerId::new(&owner_id);
        Trigger {
            id,
            owner_id,
            reference_id: None,
            name: None,
            description: None,
            created_at: Utc::now(),
            emit: Vec::default(),
            payload: Payload::default(),
            status: Status::Active,
            schedule: Some(sched),
            hidden_last_invoked_at: None,
        }
    }

    #[test]
    fn future_ticks_parsing_cron() -> Result<(), TriggerError> {
        let cron_pattern = "0 5 * * * *"; // fifth minute of every hour
        let schedule = create_cron_schedule(cron_pattern, 0);

        let mut result = TriggerFutureTicks::from_schedule(&schedule, None)?;
        assert!(matches!(result, TriggerFutureTicks::CronPattern { .. }));

        assert!(result.peek().is_some());
        let TriggerFutureTicks::CronPattern {
            mut next_ticks,
            remaining: remaining_events_limit,
        } = result else {
            panic!("Should never get here!");
        };

        assert_eq!(remaining_events_limit, None);

        assert!(next_ticks.peek().unwrap() > &Utc::now());
        Ok(())
    }

    #[test]
    fn future_ticks_parsing_cron_with_limits() -> Result<(), TriggerError> {
        //  sec  min   hour   day of month   month   day of week   year
        //  A specific second in the future, this should yield exactly one time
        // point. FIXME: This will fail soon, see https://github.com/zslayton/cron/issues/97

        let cron_pattern = "0 5 4 2 6 * 2040"; // fifth minute of every hour
        let schedule = create_cron_schedule(cron_pattern, 4);

        let mut result = TriggerFutureTicks::from_schedule(&schedule, None)?;
        assert!(matches!(result, TriggerFutureTicks::CronPattern { .. }));

        if let TriggerFutureTicks::CronPattern {
            remaining: remaining_events_limit,
            ..
        } = result
        {
            assert_eq!(remaining_events_limit, Some(4));
        } else {
            panic!("Should never get here!");
        }

        assert!(result.peek().is_some());
        assert!(result.advance_and_peek().is_none());
        assert!(result.peek().is_none());
        Ok(())
    }

    #[test]
    fn future_ticks_parsing_run_at() -> Result<(), TriggerError> {
        // generating some time points, one in the past, and three in the
        // future.
        let timepoints = vec![
            parse_iso8601("PT-1M").unwrap(), /* 1 minute ago (in the past) */
            parse_iso8601("PT2M").unwrap(),
            parse_iso8601("PT3M").unwrap(),
        ];

        let schedule = create_run_at(timepoints);

        let mut result = TriggerFutureTicks::from_schedule(&schedule, None)?;
        assert!(matches!(result, TriggerFutureTicks::RunAt { .. }));

        if let TriggerFutureTicks::RunAt { ref run_at, .. } = result {
            assert_eq!(run_at.len(), 2);
        } else {
            panic!("Should never get here!");
        }

        assert!(result.peek().is_some());
        assert!(result.advance_and_peek().is_some());
        assert!(result.peek().is_some());
        assert!(result.advance_and_peek().is_none());
        assert!(result.peek().is_none());

        Ok(())
    }

    #[test]
    fn temporal_state_advance() -> Result<(), TriggerError> {
        let cron = create_cron_schedule("* * * * * *", 0);
        let trigger = create_trigger(cron);
        let trigger_id = trigger.id.clone();
        let mut map = ActiveTriggerMap::default();
        map.add_or_update(trigger, false)?;
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick1 = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(0, temporal_states.len());
        let tick2 = map.advance(&trigger_id).unwrap();
        assert!(tick2 > tick1);
        // we should get the same (tick2) if we rebuilt the state
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick2_again = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(tick2, tick2_again);

        // rebuilding the temporal state doesn't advance anything even if time
        // passes
        std::thread::sleep(Duration::from_secs(2));
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick2_again = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(tick2, tick2_again);
        Ok(())
    }
}
