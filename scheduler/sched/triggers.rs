use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    iter::Peekable,
    str::FromStr,
};

use chrono_tz::{Tz, UTC};
use thiserror::Error;

use chrono::{DateTime, Utc};
use cron::{OwnedScheduleIterator, Schedule as CronSchedule};
use proto::trigger_proto::{self, Trigger};
use tracing::info;

#[derive(Error, Debug)]
pub(crate) enum TriggerError {
    #[error("Cannot parse cron expression")]
    CronParseError(#[from] cron::error::Error),
    #[error("Unrecognized timezone '{0}' was supplied, are you sure this is an IANA timezone?")]
    InvalidTimezone(String),
    #[error("Trigger '{0}' should not have passed validation!")]
    MalformedTrigger(String),
}

// message Trigger {
//   string id = 1;
//   string owner_id = 2;
//   /// [Future] User supplied identifier, unique per owner account
//   optional string reference_id = 3;
//   optional string name = 4;
//   optional string description = 5;
//   google.protobuf.Timestamp created_at = 6;
//   // TODO: Explore having multiple endpoints with independent status
//   Endpoint endpoint = 7;
//   Payload payload = 8;
//   google.protobuf.Duration timeout = 9;
//   Schedule schedule = 10;
//   TriggerStatus status = 11;
//   OnStatusHandler on_success = 12;
//   OnStatusHandler on_failure = 13;
//   RetryPolicy event_retry_policy = 14;
// }

///
/// Maintains the set of `active` triggers in memory. Expired triggers are
/// evicted to save space.
///
/// dirty_triggers holds the set of triggers that has been updated since the
/// last time the spinner has looked at it. The spinner resets the set after
/// reloading.
#[derive(Default)]
pub(crate) struct ActiveTriggerMap {
    state: HashMap<String, ActiveTrigger>,
    /// The set of trigger Ids that has been updated
    dirty: bool,
}

impl ActiveTriggerMap {
    /// Inserts or updates a trigger if exists
    pub fn add_or_update(
        &mut self,
        trigger: Trigger,
    ) -> Result<(), TriggerError> {
        let trigger_id = trigger.id.clone();
        let active_trigger = ActiveTrigger::try_from(trigger)?;
        self.state.insert(trigger_id, active_trigger);
        self.mark_dirty();
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
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

    /*
     * Advance ensures that the next tick is not the same as current tick.
     * I hear you asking, why do we need that?
     *
     * We need this because:
     * - We don't want to advance everything when we build a new temporal states,
     *   as this will incorrectly advance triggers that are still due.
     * - When temporal state is created, we only peek(), this makes the first iteration
     *   a bit awkward, when we advance after executing the trigger, advance() will
     *   return the same time point because we have never next()ed it. This _hack_
     *   ensures that we will fast-forward in this rare case.
     * - This also ensures that, for any reason, we skip duplicates in the run_at
     *   list if we didn't catch this in validation.
     */
    pub fn advance(&mut self, trigger_id: &String) -> Option<DateTime<Tz>> {
        self.state
            .get_mut(trigger_id)
            .and_then(|trigger| trigger.advance())
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
    pub trigger_id: String,
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
        remaining_events_limit: Option<i64>,
    },
    RunAt(BinaryHeap<Reverse<DateTime<Tz>>>),
}

impl TriggerFutureTicks {
    pub fn from_proto(
        schedule_proto: &trigger_proto::schedule::Schedule,
    ) -> Result<Self, TriggerError> {
        match schedule_proto {
            | trigger_proto::schedule::Schedule::Cron(cron) => {
                let raw_pattern = cron.cron.clone();
                let cron_schedule = CronSchedule::from_str(&raw_pattern)?;
                let tz: Tz = cron.timezone.parse().map_err(|_| {
                    TriggerError::InvalidTimezone(cron.timezone.clone())
                })?;
                let next_ticks = cron_schedule.upcoming_owned(tz).peekable();
                // TODO: This should be remaining_events_limit, or instead,
                // change the spec to set an end date.
                let events_limit = if cron.events_limit > 0 {
                    Some(cron.events_limit)
                } else {
                    None
                };
                Ok(TriggerFutureTicks::CronPattern {
                    next_ticks,
                    remaining_events_limit: events_limit,
                })
            }
            | trigger_proto::schedule::Schedule::RunAt(run_at) => {
                let mut ticks = BinaryHeap::new();
                for ts in run_at.run_at.iter() {
                    let parsed: DateTime<Tz> =
                        DateTime::parse_from_str(ts, "%+")
                            .unwrap()
                            .with_timezone(&UTC);
                    if parsed < Utc::now() {
                        // TODO: Remove this log line
                        info!("Tick of trigger is in the past, skipping...");
                    } else {
                        // Reversed to make this min-heap
                        ticks.push(Reverse(parsed));
                    }
                }
                Ok(TriggerFutureTicks::RunAt(ticks))
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

    fn peek_or_next(&mut self, next: bool) -> Option<DateTime<Tz>> {
        match self {
            // We hit the run limit.
            | Self::CronPattern {
                remaining_events_limit: Some(events_limit),
                ..
            } if *events_limit == 0 => None,
            // We might have more runs
            | Self::CronPattern {
                next_ticks,
                remaining_events_limit,
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
            | Self::RunAt(ticks) => {
                if next {
                    ticks.pop().map(|f| f.0)
                } else {
                    ticks.peek().map(|f| f.0)
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

impl TryFrom<Trigger> for ActiveTrigger {
    type Error = TriggerError;

    fn try_from(trigger: Trigger) -> Result<Self, Self::Error> {
        // Do we have a cron pattern or a set of time points?
        let k = trigger.schedule.as_ref().ok_or_else(|| {
            TriggerError::MalformedTrigger(trigger.id.clone())
        })?;
        let b = k.schedule.as_ref().unwrap();
        let ticks = TriggerFutureTicks::from_proto(b)?;
        // We assume that Trigger.schedule is never None
        Ok(Self {
            inner: trigger,
            ticks,
        })
    }
}

impl ActiveTrigger {
    pub fn get(&self) -> &Trigger {
        &self.inner
    }
    pub fn peek(&mut self) -> Option<DateTime<Tz>> {
        self.ticks.peek()
    }
    pub fn advance(&mut self) -> Option<DateTime<Tz>> {
        self.ticks.advance_and_peek()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::{Add, Sub};
    use std::time::Duration;

    use super::*;
    use trigger_proto::{Cron, RunAt, TriggerStatus};

    fn create_cron_schedule(
        pattern: &str,
        events_limit: i64,
    ) -> proto::trigger_proto::Schedule {
        proto::trigger_proto::Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::Cron(Cron {
                cron: pattern.into(),
                timezone: "Europe/London".into(),
                events_limit,
            })),
        }
    }

    fn create_run_at(
        timepoints: Vec<String>,
    ) -> proto::trigger_proto::Schedule {
        proto::trigger_proto::Schedule {
            schedule: Some(trigger_proto::schedule::Schedule::RunAt(RunAt {
                run_at: timepoints,
            })),
        }
    }

    fn create_trigger(sched: proto::trigger_proto::Schedule) -> Trigger {
        let id = format!("trig_{}", rand::random::<u64>());
        Trigger {
            id,
            owner_id: "asoli".to_owned(),
            reference_id: None,
            name: None,
            description: None,
            created_at: None,
            endpoint: None,
            payload: None,
            timeout: None,
            status: TriggerStatus::Active.into(),
            event_retry_policy: None,
            on_success: None,
            on_failure: None,
            schedule: Some(sched),
        }
    }

    #[test]
    fn future_ticks_parsing_cron() -> Result<(), TriggerError> {
        let cron_pattern = "0 5 * * * *"; // fifth minute of every hour
        let schedule_proto = create_cron_schedule(cron_pattern, 0);

        let mut result =
            TriggerFutureTicks::from_proto(&schedule_proto.schedule.unwrap())?;
        assert!(matches!(result, TriggerFutureTicks::CronPattern { .. }));

        assert!(result.peek().is_some());
        let TriggerFutureTicks::CronPattern {
            mut next_ticks,
            remaining_events_limit,
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
        //  A specific second in the future, this should yield exactly one time point.
        let cron_pattern = "0 5 4 2 3 * 2040"; // fifth minute of every hour
        let schedule_proto = create_cron_schedule(cron_pattern, 4);

        let mut result =
            TriggerFutureTicks::from_proto(&schedule_proto.schedule.unwrap())?;
        assert!(matches!(result, TriggerFutureTicks::CronPattern { .. }));

        if let TriggerFutureTicks::CronPattern {
            remaining_events_limit,
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
        // generating some time points, one in the past, and three in the future.
        let mut timepoints = vec![];
        let k = Utc::now().sub(chrono::Duration::seconds(10));
        timepoints.push(format!("{}", k.format("%+")));
        let k = Utc::now().add(chrono::Duration::seconds(10));
        timepoints.push(format!("{}", k.format("%+")));
        let k = Utc::now().add(chrono::Duration::seconds(20));
        timepoints.push(format!("{}", k.format("%+")));

        let schedule_proto = create_run_at(timepoints);

        let mut result =
            TriggerFutureTicks::from_proto(&schedule_proto.schedule.unwrap())?;
        assert!(matches!(result, TriggerFutureTicks::RunAt { .. }));

        if let TriggerFutureTicks::RunAt(ref points) = result {
            assert_eq!(points.len(), 2);
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
        map.add_or_update(trigger)?;
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

        // rebuilding the temporal state doesn't advance anything even if time passes
        std::thread::sleep(Duration::from_secs(2));
        let mut temporal_states = map.build_temporal_state();
        assert_eq!(1, temporal_states.len());
        let tick2_again = temporal_states.pop().unwrap().0.next_tick;
        assert_eq!(tick2, tick2_again);
        Ok(())
    }
}
