use std::collections::HashMap;
use std::fmt::Display;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::webhook::Webhook;
use crate::model::ValidShardedId;
use crate::timeutil::iso8601_dateformat_vec_serde;
use crate::types::{ProjectId, TriggerId};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trigger {
    pub id: TriggerId,
    pub project: ValidShardedId<ProjectId>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
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
            updated_at: self.updated_at,
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
            updated_at: self.updated_at,
            emit: self.emit.clone(),
            reference: self.reference.clone(),
            schedule: self.schedule.clone(),
            status: self.status.clone(),
            last_invoked_at: self.last_invoked_at,
        }
    }

    pub fn update(
        &mut self,
        new_name: String,
        new_description: Option<String>,
        new_reference: Option<String>,
        new_payload: Option<Payload>,
        new_schedule: Option<Schedule>,
        new_emit: Vec<Emit>,
    ) {
        self.updated_at = Some(Utc::now());

        self.name = new_name;
        self.description = new_description;
        self.reference = new_reference;
        self.payload = new_payload;
        self.schedule = new_schedule;
        self.emit = new_emit;
        self.status = if self.schedule.is_some() {
            Status::Scheduled
        } else {
            Status::OnDemand
        };
        // NOTE: we leave last_invoked_at as is.
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
    pub updated_at: Option<DateTime<Utc>>,
    pub emit: Vec<Emit>,
    pub reference: Option<String>,
    pub schedule: Option<Schedule>,
    pub status: Status,
    pub last_invoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Scheduled,
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
        [Self::Scheduled, Self::Paused].contains(self)
    }

    pub fn cancelleable(&self) -> bool {
        [Self::Scheduled, Self::Paused, Self::OnDemand].contains(self)
    }

    pub fn as_operation(&self) -> String {
        match self {
            | Status::Scheduled => "resume",
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
#[serde(untagged)]
pub enum Schedule {
    Recurring(Recurring),
    RunAt(RunAt),
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RunAt {
    #[serde(with = "iso8601_dateformat_vec_serde")]
    pub timepoints: Vec<DateTime<Tz>>,
    // Ignored if set through the API.
    pub remaining: Option<u64>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recurring {
    pub cron: Option<String>,
    pub timezone: String,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
}

impl Default for Recurring {
    fn default() -> Self {
        Self {
            cron: None,
            timezone: "Etc/UTC".to_owned(),
            limit: None,
            remaining: None,
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
pub struct Event {
    #[serde(rename = "type")]
    pub _kind: MustBe!("event"),
    event: String,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payload {
    pub headers: HashMap<String, String>,
    pub content_type: String,
    pub body: String,
}
