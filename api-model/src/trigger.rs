use chrono::{DateTime, Utc};
#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
#[cfg(feature = "validation")]
use validator::Validate;

use super::{Action, Payload, Schedule};
use crate::{Recurring, RunAt, Webhook};

#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto),
    proto(target = "proto::scheduler_proto::ListTriggersFilter")
)]
pub struct TriggersFilter {
    #[serde(default)]
    #[cfg_attr(feature = "dto", proto(name = "statuses"))]
    pub status: Vec<TriggerStatus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "client", non_exhaustive)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::trigger_proto::TriggerStatus")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum TriggerStatus {
    Scheduled,
    OnDemand,
    Expired,
    Cancelled,
    Paused,
}

impl std::fmt::Display for TriggerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_variant::to_variant_name(self).unwrap())
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::trigger_proto::Trigger")
)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields))]
pub struct Trigger {
    // Ids are meant to be internal only, so they are neither accepted as input
    // or outputed in the API. This is here just for IntoProto to work
    #[cfg_attr(feature = "server", serde(skip))]
    #[cfg(feature = "dto")]
    pub id: Option<lib::types::TriggerId>,
    #[cfg_attr(
        feature = "validation",
        validate(length(
            min = 2,
            max = 64,
            message = "name must be between 2 and 64 characters if set"
        ))
    )]
    #[cfg_attr(feature = "dto", proto(required))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[cfg_attr(feature = "validation", validate)]
    pub action: Option<Action>,
    #[cfg_attr(feature = "validation", validate)]
    pub schedule: Option<Schedule>,
    pub status: Option<TriggerStatus>,
    pub last_ran_at: Option<DateTime<Utc>>,
    #[cfg_attr(feature = "validation", validate)]
    pub payload: Option<Payload>,
    // Estimate of timepoints of the next runs (up to 5 runs)
    #[cfg_attr(
        feature = "dto",
        proto(required),
        from_proto(map = "vec_datetime_from"),
        into_proto(map = "vec_datetime_into")
    )]
    pub estimated_future_runs: Option<Vec<DateTime<Utc>>>,
}

#[cfg(feature = "dto")]
// Quick-n-dirty hack because `dto` doesn't properly handle Option<Vec<_>>
fn vec_datetime_from(
    input: Vec<proto::common::DateTime>,
) -> Vec<chrono::DateTime<Utc>> {
    input.into_iter().map(Into::into).collect()
}

#[cfg(feature = "dto")]
fn vec_datetime_into(
    input: Vec<chrono::DateTime<Utc>>,
) -> Vec<proto::common::DateTime> {
    input.into_iter().map(Into::into).collect()
}

impl Trigger {
    /// Returns the webhook if the action is a webhook
    pub fn webhook(&self) -> Option<&Webhook> {
        match self.action.as_ref() {
            | Some(Action::Webhook(webhook)) => Some(webhook),
            | _ => None,
        }
    }

    /// Returns the recurring schedule if the schedule is of type `recurring`
    pub fn recurring(&self) -> Option<&Recurring> {
        match self.schedule.as_ref() {
            | Some(Schedule::Recurring(r)) => Some(r),
            | _ => None,
        }
    }

    /// Returns the run_at schedule if the schedule is of type `timepoints`
    pub fn run_at(&self) -> Option<&RunAt> {
        match self.schedule.as_ref() {
            | Some(Schedule::RunAt(r)) => Some(r),
            | _ => None,
        }
    }
}

#[cfg(all(test, feature = "validation"))]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::*;

    #[test]
    fn validate_install_trigger_01() -> Result<()> {
        std::env::set_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION", "true");

        let request = json!(
          {
            "schedule": {
              "type": "recurring",
              "cron": "*/3 * * * * *",
              "limit": 5
            },
            "action": {
              "url": "http://localhost:3000/action",
              "timeout_s": 10,
              "retry": {
                "delay_s": 100
              }

            }
          }
        );

        let parsed: Trigger = serde_json::from_value(request)?;
        parsed.validate()?;
        std::env::remove_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION");
        Ok(())
    }
}
