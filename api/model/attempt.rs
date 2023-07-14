use std::time::Duration;

use chrono::{DateTime, Utc};
use dto::FromProto;
use lib::types::AttemptLogId;
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};

#[derive(Clone, FromProto, Debug, Serialize, PartialEq)]
#[proto(target = "proto::attempt_proto::ActionAttemptLog")]
#[skip_serializing_none]
pub struct ActionAttemptLog {
    #[proto(required)]
    pub id: AttemptLogId,
    pub status: AttemptStatus,
    #[proto(required)]
    pub details: AttemptDetails,
    #[proto(required)]
    pub created_at: DateTime<Utc>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, FromProto, Clone, Serialize, PartialEq)]
#[proto(target = "proto::attempt_proto::WebhookAttemptDetails")]
pub struct WebhookAttemptDetails {
    pub response_code: Option<i32>,
    #[proto(map_from_proto = "Duration::from_secs")]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub response_latency_s: Duration,
    pub error_message: Option<String>,
}

#[derive(Debug, FromProto, Clone, Serialize, PartialEq)]
#[proto(target = "proto::attempt_proto::AttemptDetails", oneof = "details")]
#[serde(untagged)]
pub enum AttemptDetails {
    #[proto(name = "Webhook")]
    WebhookAttemptDetails(WebhookAttemptDetails),
}

#[derive(Debug, FromProto, Clone, Serialize, PartialEq)]
#[proto(target = "proto::attempt_proto::AttemptStatus")]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Succeeded,
    Failed,
}
