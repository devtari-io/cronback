use std::time::Duration;

use chrono::{DateTime, Utc};
#[cfg(feature = "dto")]
use dto::FromProto;
#[cfg(feature = "dto")]
use lib::types::AttemptLogId;
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};

#[cfg(not(feature = "dto"))]
use crate::AttemptLogId;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::ActionAttemptLog")
)]
#[skip_serializing_none]
pub struct ActionAttemptLog {
    #[cfg_attr(feature = "dto", proto(required))]
    pub id: AttemptLogId,
    pub status: AttemptStatus,
    #[cfg_attr(feature = "dto", proto(required))]
    pub details: AttemptDetails,
    #[cfg_attr(feature = "dto", proto(required))]
    pub created_at: DateTime<Utc>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::WebhookAttemptDetails")
)]
pub struct WebhookAttemptDetails {
    pub response_code: Option<i32>,
    #[cfg_attr(feature = "dto", proto(map_from_proto = "Duration::from_secs"))]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub response_latency_s: Duration,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::AttemptDetails", oneof = "details")
)]
#[serde(untagged)]
pub enum AttemptDetails {
    #[cfg_attr(feature = "dto", proto(name = "Webhook"))]
    WebhookAttemptDetails(WebhookAttemptDetails),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::AttemptStatus")
)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Succeeded,
    Failed,
}
