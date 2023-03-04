use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use shared::model_util::generate_model_id;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(tag = "object")]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct Trigger {
    #[serde(skip_deserializing)]
    pub id: String,
    #[validate(length(
        min = 2,
        max = 1000,
        message = "name must be between 2 and 1000 characters if set"
    ))]
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    // pub reference_id: Option<String>,
    #[validate(length(
        min = 0,
        max = 1048576,
        message = "Payload must be under 1MiB"
    ))]
    pub payload: Option<String>,
    pub headers: HashMap<String, String>,
    pub content_type: String,
    pub cron: Option<String>,
    // TODO: Validate this follows IANA tz names.
    pub cron_timezone: String,
    pub cron_events_limit: u64,
    // TODO: Validate that this is valid ISO 8601 dates and that they are in order with no
    // duplicates.
    pub run_at: Option<Vec<String>>,
    #[validate(range(
        min = 1.0,
        max = 30.0,
        message = "Timeout must be between 1 and 30 seconds"
    ))]
    pub timeout_s: f64,
    #[serde(skip_deserializing)]
    pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
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

// fn from_request(
impl Default for Trigger {
    fn default() -> Self {
        Self {
            id: generate_model_id("trig"),
            name: None,
            created_at: Utc::now(),
            payload: None,
            headers: Default::default(),
            content_type: "application/json; charset=utf-8".to_string(),
            cron: None,
            cron_timezone: "Etc/UTC".to_string(),
            // 0 means no limit
            cron_events_limit: 0,
            run_at: None,
            timeout_s: 5.0,
            status: Default::default(),
        }
    }
}
