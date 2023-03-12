use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use chrono::DateTime;
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};
use validator::{Validate, ValidationError};

use crate::timeutil::iso8601_dateformat_serde;
use crate::timeutil::iso8601_dateformat_vec_serde;

use crate::types::{OwnerId, TriggerId};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: TriggerId,

    pub owner_id: OwnerId,

    pub name: Option<String>,

    pub description: Option<String>,

    #[serde(with = "iso8601_dateformat_serde")]
    pub created_at: DateTime<Tz>,

    pub reference_id: Option<String>,

    pub payload: Payload,

    pub schedule: Option<Schedule>,

    pub emit: Vec<Emit>,

    pub status: Status,
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
    #[serde(with = "iso8601_dateformat_vec_serde")]
    pub run_at: Vec<DateTime<Tz>>,
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
    DELETE,
    GET,
    HEAD,
    PATCH,
    POST,
    PUT,
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

fn validate_timezone(cron_timezone: &String) -> Result<(), ValidationError> {
    // validate timezone
    let tz: Result<Tz, _> = cron_timezone.parse();
    if tz.is_err() {
        return Err(validation_error(
            "unrecognized_cron_timezone",
            format!(
                "Timezone unrecognized '{cron_timezone}'. A valid IANA timezone string is required",
            )
        ));
    };
    Ok(())
}

fn validate_timeout(timeout: &Duration) -> Result<(), ValidationError> {
    if timeout.as_secs_f64() < 1.0 || timeout.as_secs_f64() > 30.0 {
        return Err(validation_error(
            "invalid_timeout",
            "Timeout must be between 1.0 and 30.0 seconds".to_string(),
        ));
    };
    Ok(())
}

fn validation_error(code: &'static str, message: String) -> ValidationError {
    let mut validation_e = ValidationError::new(code);
    validation_e.message = Some(message.into());
    validation_e
}
