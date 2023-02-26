use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    iter::Peekable,
    str::FromStr,
};

use chrono_tz::Tz;
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
    dirty_triggers: HashSet<String>,
}

impl ActiveTriggerMap {
    /// Inserts or updates a trigger if exists
    pub fn add_or_update(
        &mut self,
        trigger: Trigger,
    ) -> Result<(), TriggerError> {
        let trigger_id = trigger.id.clone();
        let active_trigger = ActiveTrigger::try_from(trigger)?;
        self.state.insert(trigger_id.clone(), active_trigger);
        self.trigger_updated(trigger_id);
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty_triggers.is_empty()
    }

    pub fn reset_dirty(&mut self) {
        self.dirty_triggers.clear();
    }

    pub fn clear(&mut self) {
        self.state.clear();
        self.dirty_triggers.clear();
    }

    pub fn triggers_iter(
        &self,
    ) -> std::collections::hash_map::Values<'_, String, ActiveTrigger> {
        self.state.values()
    }

    //// PRIVATE
    fn trigger_updated(&mut self, trigger_id: String) {
        self.dirty_triggers.insert(trigger_id);
    }
}

/// Metadata exclusively owned by the spinner, keeps the Id of the installed
/// trigger along with its next tick.
///
/// The spinner maintains a max-heap of those jobs to determine which Triggers
/// need to be evaluated at each loop.
pub(crate) struct TemporalState {
    #[allow(unused)]
    pub trigger_id: String,
    // timestamp in ms of the next tick
    pub next_tick: i64,
}

impl PartialOrd for TemporalState {
    fn partial_cmp(&self, other: &TemporalState) -> Option<std::cmp::Ordering> {
        self.next_tick.partial_cmp(&other.next_tick)
    }
}
impl Ord for TemporalState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next_tick.cmp(&other.next_tick)
    }
}

impl PartialEq for TemporalState {
    fn eq(&self, other: &Self) -> bool {
        self.next_tick == other.next_tick
    }
}
impl Eq for TemporalState {}

pub(crate) enum TriggerFutureTicks {
    CronPattern {
        raw_pattern: String,
        cron_schedule: CronSchedule,
        next_ticks: Peekable<OwnedScheduleIterator<Tz>>,
        remaining_events_limit: Option<i64>,
    },
    RunAt(BinaryHeap<Reverse<i64>>),
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
                return Ok(TriggerFutureTicks::CronPattern {
                    raw_pattern,
                    cron_schedule,
                    next_ticks,
                    remaining_events_limit: events_limit,
                });
            }
            | trigger_proto::schedule::Schedule::RunAt(run_at) => {
                let mut ticks = BinaryHeap::new();
                for ts in run_at.run_at.iter() {
                    let parsed = DateTime::parse_from_str(ts, "%+").unwrap();
                    if parsed < Utc::now() {
                        // TODO: Remove this log line
                        info!("Tick of trigger is in the past, skipping...");
                    } else {
                        // Reversed to make this min-heap
                        ticks.push(Reverse(parsed.timestamp()));
                    }
                }
                return Ok(TriggerFutureTicks::RunAt(ticks));
            }
        };
    }
    pub fn next(&mut self) -> Option<i64> {
        self.peek_or_next(true)
    }

    pub fn peek(&mut self) -> Option<i64> {
        self.peek_or_next(false)
    }

    pub fn is_active(&mut self) -> bool {
        self.peek().is_some()
    }

    fn peek_or_next(&mut self, next: bool) -> Option<i64> {
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
                    let n = next_ticks.next().map(|e| e.timestamp());
                    if n.is_some() {
                        if let Some(events_limit) = remaining_events_limit {
                            // consume events.
                            *events_limit -= 1;
                        }
                    }
                    n
                } else {
                    next_ticks.peek().map(|e| e.timestamp())
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
        let k = trigger.schedule.as_ref().unwrap();
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

    pub fn into_trigger(self) -> Trigger {
        self.inner
    }

    pub fn is_active(&mut self) -> bool {
        self.ticks.is_active()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::{Add, Sub};
    use std::time::Duration;

    use super::*;
    use trigger_proto::schedule::Schedule;
    use trigger_proto::{Cron, RunAt};

    #[test]
    fn future_ticks_parsing_cron() -> Result<(), TriggerError> {
        let cron_pattern = "0 5 * * * *"; // fifth minute of every hour
        let schedule_proto = Schedule::Cron(Cron {
            cron: cron_pattern.into(),
            timezone: "Europe/London".into(),
            events_limit: 0,
        });

        let mut result = TriggerFutureTicks::from_proto(&schedule_proto)?;
        assert!(matches!(result, TriggerFutureTicks::CronPattern { .. }));

        assert!(result.is_active());
        let TriggerFutureTicks::CronPattern {
            raw_pattern,
            cron_schedule,
            mut next_ticks,
            remaining_events_limit,
        } = result else {
            panic!("Should never get here!");
        };

        assert_eq!(raw_pattern, cron_pattern);
        assert_eq!(remaining_events_limit, None);
        assert_eq!(
            cron_schedule,
            CronSchedule::from_str(&raw_pattern).unwrap()
        );

        assert!(next_ticks.peek().unwrap() > &Utc::now());
        assert_eq!(
            cron_schedule,
            CronSchedule::from_str(&raw_pattern).unwrap()
        );
        Ok(())
    }

    #[test]
    fn future_ticks_parsing_cron_with_limits() -> Result<(), TriggerError> {
        //  sec  min   hour   day of month   month   day of week   year
        //  A specific second in the future, this should yield exactly one time point.
        let cron_pattern = "0 5 4 2 3 * 2040"; // fifth minute of every hour
        let schedule_proto = Schedule::Cron(Cron {
            cron: cron_pattern.into(),
            timezone: "Africa/Cairo".into(),
            events_limit: 4,
        });

        let mut result = TriggerFutureTicks::from_proto(&schedule_proto)?;
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

        assert!(result.is_active());
        assert!(result.peek().is_some());
        assert!(result.next().is_some());
        assert!(result.next().is_none());
        assert!(result.peek().is_none());
        assert!(!result.is_active());

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

        let schedule_proto = Schedule::RunAt(RunAt { run_at: timepoints });

        let mut result = TriggerFutureTicks::from_proto(&schedule_proto)?;
        assert!(matches!(result, TriggerFutureTicks::RunAt { .. }));

        if let TriggerFutureTicks::RunAt(ref points) = result {
            assert_eq!(points.len(), 2);
        } else {
            panic!("Should never get here!");
        }

        assert!(result.is_active());
        assert!(result.peek().is_some());
        assert!(result.next().is_some());
        assert!(result.next().is_some());
        assert!(result.next().is_none());
        assert!(result.peek().is_none());
        assert!(!result.is_active());

        Ok(())
    }
}
