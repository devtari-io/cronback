use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::{Validate, ValidationError};

use super::webhook::Webhook;
use crate::model::ValidShardedId;
use crate::timeutil::iso8601_dateformat_vec_serde;
use crate::types::{ProjectId, TriggerId};
use crate::validation::{validate_timezone, validation_error};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    pub id: TriggerId,
    pub project: ValidShardedId<ProjectId>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reference: Option<String>,
    pub payload: Option<Payload>,
    pub schedule: Option<Schedule>,
    pub emit: Vec<Emit>,
    pub status: Status,
    pub last_invoked_at: Option<DateTime<Utc>>,
}

impl Trigger {
    pub fn alive(&self) -> bool {
        self.status.alive()
    }

    pub fn into_manifest(self) -> TriggerManifest {
        TriggerManifest {
            id: self.id,
            project: self.project,
            name: self.name,
            description: self.description,
            created_at: self.created_at,
            emit: self.emit,
            reference: self.reference,
            schedule: self.schedule,
            status: self.status,
            last_invoked_at: self.last_invoked_at,
        }
    }

    pub fn get_manifest(&self) -> TriggerManifest {
        TriggerManifest {
            id: self.id.clone(),
            project: self.project.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            created_at: self.created_at,
            emit: self.emit.clone(),
            reference: self.reference.clone(),
            schedule: self.schedule.clone(),
            status: self.status.clone(),
            last_invoked_at: self.last_invoked_at,
        }
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriggerManifest {
    pub id: TriggerId,
    pub project: ValidShardedId<ProjectId>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub emit: Vec<Emit>,
    pub reference: Option<String>,
    pub schedule: Option<Schedule>,
    pub status: Status,
    pub last_invoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Status {
    #[default]
    Active,
    OnDemand,
    Expired,
    Cancelled,
    Paused,
}

impl Status {
    // Alive means that it should continue to live in the spinner map. A paused
    // trigger is considered alive, but it won't be invoked. We will advance
    // its clock as if it was invoked though.
    pub fn alive(&self) -> bool {
        [Self::Active, Self::Paused].contains(self)
    }

    pub fn cancelleable(&self) -> bool {
        [Self::Active, Self::Paused, Self::OnDemand].contains(self)
    }

    pub fn as_operation(&self) -> String {
        match self {
            | Status::Active => "resume",
            | Status::Expired => "expire",
            | Status::Cancelled => "cancel",
            | Status::Paused => "pause",
            | _ => "invalid",
        }
        .to_owned()
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Schedule {
    Recurring(Cron),
    RunAt(RunAt),
}

#[skip_serializing_none]
#[serde_as]
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, Validate, PartialEq,
)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RunAt {
    #[validate(
        length(
            min = 1,
            max = 5000,
            message = "Reached maximum number of timepoint events in the same \
                       trigger"
        ),
        custom = "validate_run_at"
    )]
    #[serde(with = "iso8601_dateformat_vec_serde")]
    pub timepoints: Vec<DateTime<Tz>>,
    // TODO: Reject if set through the API.
    pub remaining: u64,
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
    pub timezone: String,
    pub limit: u64,
    // TODO: Reject if set through the API.
    pub remaining: u64,
}

impl Default for Cron {
    fn default() -> Self {
        Self {
            cron: None,
            timezone: "Etc/UTC".to_owned(),
            limit: 0,
            remaining: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
//#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Emit {
    Event(Event),
    Webhook(Webhook),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(rename = "type")]
    pub _kind: MustBe!("event"),
    event: String,
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
            | Emit::Event(_) => Ok(()),
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
