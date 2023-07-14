use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trigger {
    name: Option<String>,
    description: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    action: Option<Action>,
    schedule: Option<Schedule>,
    status: Option<TriggerStatus>,
    last_ran_at: Option<DateTime<Utc>>,
    payload: Option<Payload>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerStatus {
    Scheduled,
    OnDemand,
    Expired,
    Cancelled,
    Paused,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Delete,
    Get,
    Head,
    Patch,
    Post,
    Put,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
enum Action {
    Webhook(Webhook),
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Webhook {
    // allows an optional "type" field to be passed in. This enables other
    // variants of action to be differentiated.
    #[serde(rename = "type")]
    _kind: MustBe!("webhook"),
    pub url: Option<String>,
    pub http_method: HttpMethod,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub timeout_s: std::time::Duration,
    // None means no retry
    pub retry: Option<RetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum RetryConfig {
    SimpleRetry(SimpleRetry),
    ExponentialBackoffRetry(ExponentialBackoffRetry),
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleRetry {
    #[serde(rename = "type")]
    _kind: MustBe!("simple"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub delay_s: Duration,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExponentialBackoffRetry {
    #[serde(rename = "type")]
    _kind: MustBe!("exponential_backoff"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub delay_s: Duration,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub max_delay_s: Duration,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
enum Schedule {
    Recurring(Recurring),
    RunAt(RunAt),
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Recurring {
    pub cron: Option<String>,
    pub timezone: Option<String>,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RunAt {
    pub timepoints: Vec<DateTime<FixedOffset>>,
    pub remaining: Option<u64>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Payload {
    pub headers: BTreeMap<String, String>,
    pub content_type: Option<String>,
    pub body: Option<String>,
}
