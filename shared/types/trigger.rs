use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::{Validate, ValidationError};

use super::webhook::Webhook;
use crate::timeutil::iso8601_dateformat_vec_serde;
use crate::types::{OwnerId, TriggerId};
use crate::validation::{validate_timezone, validation_error};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    pub id: TriggerId,

    pub owner_id: OwnerId,

    pub name: Option<String>,

    pub description: Option<String>,

    pub created_at: DateTime<Utc>,

    pub reference_id: Option<String>,

    pub payload: Payload,

    pub schedule: Option<Schedule>,

    pub emit: Vec<Emit>,

    pub status: Status,

    pub hidden_last_invoked_at: Option<DateTime<Utc>>,
    //pub hidden_remaining_cron_events: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Status {
    Active,
    Expired,
    Canceled,
    Paused,
}

impl Default for Status {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Schedule {
    Recurring(Cron),
    RunAt(RunAt),
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(transparent)]
pub struct RunAt {
    #[validate(
        length(
            min = 1,
            max = 5000,
            message = "Reached maximum number of run_at events in the same \
                       trigger"
        ),
        custom = "validate_run_at"
    )]
    #[serde(with = "iso8601_dateformat_vec_serde")]
    pub run_at: Vec<DateTime<Tz>>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Cron {
    #[validate(custom = "validate_cron", required)]
    pub cron: Option<String>,

    #[validate(custom = "validate_timezone")]
    pub cron_timezone: String,

    pub cron_events_limit: u64,
}

impl Default for Cron {
    fn default() -> Self {
        Self {
            cron: None,
            cron_timezone: "Etc/UTC".to_owned(),
            cron_events_limit: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Emit {
    Webhook(Webhook),
    //Tunnel(Url),
    //Event(Event),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    #[validate(length(
        max = 30,
        message = "Max number of headers reached (>=30)"
    ))]
    pub headers: HashMap<String, String>,
    pub content_type: String,
    #[validate(length(
        min = 0,
        max = 1048576,
        message = "Payload must be under 1MiB"
    ))]
    pub body: String,
}

impl Default for Payload {
    fn default() -> Self {
        Self {
            headers: Default::default(),
            content_type: "application/json; charset=utf-8".to_owned(),
            body: Default::default(),
        }
    }
}

/// --- Validators ---
impl Validate for Emit {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Emit::Webhook(webhook) => webhook.validate(),
        }
    }
}

impl Validate for Schedule {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Schedule::Recurring(cron) => cron.validate(),
            | Schedule::RunAt(run_at) => run_at.validate(),
        }
    }
}

fn validate_cron(cron_pattern: &String) -> Result<(), ValidationError> {
    if CronSchedule::from_str(cron_pattern).is_err() {
        return Err(validation_error(
            "invalid_cron_pattern",
            format!("Invalid cron_pattern '{}'", cron_pattern),
        ));
    }
    Ok(())
}

// Validate that run_at has no duplicates.
fn validate_run_at(run_at: &Vec<DateTime<Tz>>) -> Result<(), ValidationError> {
    let mut ts = HashSet::new();
    for timepoint in run_at {
        if ts.contains(timepoint) {
            // Duplicate found!
            return Err(validation_error(
                "duplicate_run_at_value",
                format!("Duplicate value '{timepoint}'"),
            ));
        } else {
            ts.insert(timepoint);
        }
    }
    Ok(())
}
