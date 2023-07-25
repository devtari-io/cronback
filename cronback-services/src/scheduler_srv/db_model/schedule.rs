use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::iter::Peekable;
use std::str::FromStr;

use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::{Tz, UTC};
use cron::{OwnedScheduleIterator, Schedule as CronSchedule};
use dto::{FromProto, IntoProto};
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

use crate::scheduler_srv::error::TriggerError;

#[derive(
    Debug,
    Clone,
    FromProto,
    IntoProto,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    FromJsonQueryResult,
)]
#[proto(target = "proto::trigger_proto::Schedule")]
pub enum Schedule {
    Recurring(Recurring),
    RunAt(RunAt),
}
impl Schedule {
    pub fn estimated_future_runs(
        &self,
        after: DateTime<Utc>,
        count: usize,
    ) -> Vec<DateTime<Utc>> {
        let schedule_iter = ScheduleIter::from_schedule(self, Some(after))
            .expect("Failed to parse schedule");
        schedule_iter
            .take(count)
            .map(|x| x.with_timezone(&Utc))
            .collect()
    }

    pub fn limit(&self) -> Option<u64> {
        match self {
            | Self::Recurring(recurring) => recurring.limit,
            | Self::RunAt(run_at) => Some(run_at.timepoints.len() as u64),
        }
    }

    pub fn set_remaining(&mut self, remaining: Option<u64>) {
        match self {
            | Self::Recurring(recurring) => recurring.remaining = remaining,
            | Self::RunAt(run_at) => run_at.remaining = remaining,
        }
    }
}

#[derive(
    Debug,
    IntoProto,
    FromProto,
    Default,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
#[proto(target = "proto::trigger_proto::RunAt")]
pub struct RunAt {
    pub timepoints: Vec<DateTime<FixedOffset>>,
    pub remaining: Option<u64>,
}

#[derive(
    Debug, IntoProto, FromProto, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[proto(target = "proto::trigger_proto::Recurring")]
pub struct Recurring {
    pub cron: String,
    pub timezone: String,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
}

/// A type that abstracts the generator for future ticks, whether it's backed
/// by a cron patter, or a list of time points.
pub enum ScheduleIter {
    CronPattern {
        next_ticks: Peekable<OwnedScheduleIterator<Tz>>,
        remaining: Option<u64>,
    },
    RunAt {
        run_at: BinaryHeap<Reverse<DateTime<FixedOffset>>>,
        remaining: u64,
    },
}

impl ScheduleIter {
    pub fn from_schedule(
        schedule_raw: &Schedule,
        last_ran_at: Option<DateTime<Utc>>,
    ) -> Result<Self, TriggerError> {
        match schedule_raw {
            | Schedule::Recurring(cron) => {
                let raw_pattern = cron.cron.clone();
                let cron_schedule = CronSchedule::from_str(&raw_pattern)?;
                let tz: Tz = cron.timezone.parse().map_err(|_| {
                    TriggerError::InvalidTimezone(cron.timezone.clone())
                })?;
                let next_ticks = if let Some(last_ran_at) = last_ran_at {
                    cron_schedule
                        .after_owned(last_ran_at.with_timezone(&UTC))
                        .peekable()
                } else {
                    cron_schedule.upcoming_owned(tz).peekable()
                };
                let remaining = cron.remaining.or(cron.limit);
                Ok(ScheduleIter::CronPattern {
                    next_ticks,
                    remaining,
                })
            }
            | Schedule::RunAt(run_at) => {
                let mut ticks = BinaryHeap::new();
                let last_ran_at =
                    last_ran_at.unwrap_or(Utc::now()).with_timezone(&UTC);
                let mut remaining = 0;

                for ts in run_at.timepoints.iter() {
                    if *ts > last_ran_at {
                        remaining += 1;
                        // Reversed to make this min-heap
                        ticks.push(Reverse(*ts));
                    }
                }
                Ok(ScheduleIter::RunAt {
                    run_at: ticks,
                    remaining,
                })
            }
        }
    }

    // Looks at the next tick. You don't need a Peekable<Iterator> for this
    // iterator.
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
                let next_point = if next {
                    let n = run_at.pop().map(|f| f.0);
                    if n.is_some() {
                        *remaining -= 1;
                    }
                    n
                } else {
                    run_at.peek().map(|f| f.0)
                };
                // Internally, we convert everything to UTC.
                next_point.map(|f| f.with_timezone(&UTC))
            }
        }
    }
}

impl Iterator for ScheduleIter {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        self.peek_or_next(true)
    }
}

#[cfg(test)]
mod tests {

    use chrono::TimeZone;

    use super::*;

    #[test]
    fn future_ticks_parsing_cron() -> Result<(), TriggerError> {
        let pattern = "0 5 * * * *"; // fifth minute of every hour
        let schedule = Schedule::Recurring(Recurring {
            cron: pattern.to_string(),
            timezone: "Etc/UTC".into(),
            limit: None,
            remaining: None,
        });
        assert!(schedule.limit().is_none());

        let after = Utc.with_ymd_and_hms(2021, 10, 27, 0, 5, 0).unwrap();

        let result = ScheduleIter::from_schedule(&schedule, Some(after))?;
        // TODO: Consider removing this check.
        assert!(matches!(result, ScheduleIter::CronPattern { .. }));
        assert!(result.remaining().is_none());

        let mut result = result.peekable();
        assert!(result.peek().is_some());
        let head = *result.peek().unwrap();
        assert_eq!(chrono_tz::UTC, head.timezone());

        // The immediate next tick should be on 2021-10-27 01:05:00 UTC.
        let expected = chrono_tz::UTC
            .with_ymd_and_hms(2021, 10, 27, 1, 5, 0)
            .unwrap();
        assert_eq!(expected, head);
        // move the iterator
        assert_eq!(head, result.next().unwrap());

        // The next tick should be on 2021-10-27 02:05:00 UTC.
        let expected = chrono_tz::UTC
            .with_ymd_and_hms(2021, 10, 27, 2, 5, 0)
            .unwrap();
        let head = *result.peek().unwrap();
        assert_eq!(expected, head);
        assert_eq!(head, result.next().unwrap());

        Ok(())
    }

    #[test]
    fn future_ticks_parsing_cron_with_limits() -> Result<(), TriggerError> {
        //  sec  min   hour   day of month   month   day of week   year
        //  A specific second in the future, this should yield exactly one time
        // point.
        let schedule = Schedule::Recurring(Recurring {
            cron: "0 5 4 20 3 * 2040".to_owned(),
            timezone: "Etc/UTC".into(),
            limit: Some(4),
            remaining: None,
        });
        assert_eq!(4, schedule.limit().unwrap());

        let after = Utc.with_ymd_and_hms(2021, 10, 27, 0, 5, 0).unwrap();
        let mut result = ScheduleIter::from_schedule(&schedule, Some(after))?;

        assert_eq!(4, result.remaining().unwrap());

        // A specific timepoint.
        let expected = chrono_tz::UTC
            .with_ymd_and_hms(2040, 3, 20, 4, 5, 0)
            .unwrap();

        let head = result.next().unwrap();
        assert_eq!(expected, head);
        assert_eq!(3, result.remaining().unwrap());
        //  No more time points.
        assert!(result.next().is_none());
        // The remaining value is still 3 because we only ran 1 out of the limit
        // which was 4.
        assert_eq!(3, result.remaining().unwrap());

        Ok(())
    }

    #[test]
    fn future_ticks_parsing_run_at() -> Result<(), TriggerError> {
        // generating some time points, one in the past, and three in the
        // future.
        let past = chrono_tz::Egypt
            .with_ymd_and_hms(2021, 10, 27, 0, 4, 0)
            .unwrap()
            .fixed_offset();
        // <- We will start the iterator from 0:05:00
        let after_2m = chrono_tz::Egypt
            .with_ymd_and_hms(2021, 10, 27, 0, 7, 0)
            .unwrap()
            .fixed_offset();
        let after_3m = chrono_tz::Egypt
            .with_ymd_and_hms(2021, 10, 27, 0, 8, 0)
            .unwrap()
            .fixed_offset();

        let timepoints = vec![past, after_2m, after_3m];

        let schedule = Schedule::RunAt(RunAt {
            timepoints,
            remaining: None,
        });

        // Egypt was UTC+2 in 2021-10-27, so we assume that now is the 26th of
        // October at 22:05
        let after = Utc.with_ymd_and_hms(2021, 10, 26, 22, 5, 0).unwrap();
        let mut result = ScheduleIter::from_schedule(&schedule, Some(after))?;
        assert!(matches!(result, ScheduleIter::RunAt { .. }));
        assert_eq!(2, result.remaining().unwrap());
        assert_eq!(after_2m, result.next().unwrap());
        assert_eq!(1, result.remaining().unwrap());
        assert_eq!(after_3m, result.next().unwrap());
        assert_eq!(0, result.remaining().unwrap());
        assert!(result.next().is_none());
        // it won't overflow to negative.
        assert_eq!(0, result.remaining().unwrap());

        Ok(())
    }
}
