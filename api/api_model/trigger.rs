use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use proto::trigger_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};
use shared::timeutil::to_iso8601;
use validator::{Validate, ValidationError};

use shared::timeutil::iso8601_dateformat;
use shared::types::{OwnerId, TriggerId};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "object")]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    #[serde(skip_deserializing)]
    pub id: TriggerId,

    #[serde(skip_deserializing)]
    pub owner_id: OwnerId,

    pub name: Option<String>,

    pub description: Option<String>,

    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,

    pub reference_id: Option<String>,

    pub payload: Payload,

    pub schedule: Option<Schedule>,

    pub emit: Option<Emit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Schedule {
    Recurring(Cron),
    RunAt(RunAt),
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[serde(transparent)]
pub struct RunAt {
    #[validate(
        length(
            min = 1,
            max = 5000,
            message = "Reached maximum number of run_at events in the same trigger"
        ),
        custom = "validate_run_at"
    )]
    #[serde(with = "iso8601_dateformat")]
    run_at: Vec<DateTime<Tz>>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Emit {
    Webhook(Webhook),
    //Tunnel(Url),
    //Event(Event),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
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

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    // TODO validate as url
    #[validate(required)]
    pub url: Option<String>,
    pub http_method: HttpMethod,
    #[validate(custom = "validate_timeout")]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub timeout_s: std::time::Duration,
}

impl Default for Webhook {
    fn default() -> Self {
        Self {
            url: None,
            http_method: HttpMethod::POST,
            timeout_s: Duration::from_secs(5),
        }
    }
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

impl From<Payload> for proto::trigger_proto::Payload {
    fn from(value: Payload) -> Self {
        Self {
            content_type: value.content_type,
            headers: value.headers,
            body: value.body.into(),
        }
    }
}

impl From<Schedule> for proto::trigger_proto::Schedule {
    fn from(value: Schedule) -> Self {
        let schedule = match value {
            | Schedule::Recurring(cron) => {
                proto::trigger_proto::schedule::Schedule::Cron(cron.into())
            }
            | Schedule::RunAt(run_at) => {
                proto::trigger_proto::schedule::Schedule::RunAt(run_at.into())
            }
        };
        Self {
            schedule: Some(schedule),
        }
    }
}

impl From<Cron> for proto::trigger_proto::Cron {
    fn from(value: Cron) -> Self {
        Self {
            cron: value.cron.unwrap(),
            timezone: value.cron_timezone,
            events_limit: value.cron_events_limit,
        }
    }
}

impl From<RunAt> for trigger_proto::RunAt {
    fn from(value: RunAt) -> Self {
        Self {
            run_at: value.run_at.into_iter().map(|d| to_iso8601(&d)).collect(),
        }
    }
}

impl From<Emit> for proto::trigger_proto::Emit {
    fn from(value: Emit) -> Self {
        let emit = match value {
            | Emit::Webhook(webhook) => {
                trigger_proto::emit::Emit::Webhook(trigger_proto::Webhook {
                    http_method: webhook.http_method.into(),
                    url: webhook.url.unwrap(),
                    timeout_s: webhook.timeout_s.as_secs_f64(),
                })
            }
        };
        trigger_proto::Emit { emit: Some(emit) }
    }
}

impl From<HttpMethod> for i32 {
    fn from(value: HttpMethod) -> Self {
        let enum_value = match value {
            | HttpMethod::GET => trigger_proto::HttpMethod::Get,
            | HttpMethod::POST => trigger_proto::HttpMethod::Post,
            | HttpMethod::PUT => trigger_proto::HttpMethod::Put,
            | HttpMethod::DELETE => trigger_proto::HttpMethod::Delete,
            | HttpMethod::PATCH => trigger_proto::HttpMethod::Patch,
        };
        enum_value as i32
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
                format!("Duplicate value '{}'", timepoint).into(),
            ));
        } else {
            ts.insert(timepoint);
        }
    }
    Ok(())
}

fn validate_timezone(cron_timezone: &String) -> Result<(), ValidationError> {
    // validate timezone
    let tz: Result<Tz, _> = cron_timezone.parse();
    if tz.is_err() {
        return Err(validation_error(
            "unrecognized_cron_timezone",
            format!(
                "Timezone unrecognized '{}'. A valid IANA timezone string is required",
                cron_timezone
            )
        ));
    };
    Ok(())
}

fn validate_timeout(timeout: &Duration) -> Result<(), ValidationError> {
    if timeout.as_secs_f64() < 1.0 || timeout.as_secs_f64() > 30.0 {
        return Err(validation_error(
            "invalid_timeout",
            format!("Timeout must be between 1.0 and 30.0 seconds"),
        ));
    };
    Ok(())
}

fn validation_error(code: &'static str, message: String) -> ValidationError {
    let mut validation_e = ValidationError::new(code);
    validation_e.message = Some(message.into());
    validation_e
}
