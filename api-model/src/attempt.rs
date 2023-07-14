use std::time::Duration;

use chrono::{DateTime, Utc};
#[cfg(feature = "dto")]
use dto::FromProto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::Attempt")
)]
#[skip_serializing_none]
pub struct Attempt {
    pub status: AttemptStatus,
    #[cfg_attr(feature = "dto", proto(required))]
    pub details: AttemptDetails,
    pub attempt_num: u32,
    #[cfg_attr(feature = "dto", proto(required))]
    pub created_at: DateTime<Utc>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::WebhookAttemptDetails")
)]
pub struct WebhookAttemptDetails {
    pub response_code: Option<i32>,
    #[cfg_attr(feature = "dto", from_proto(map = "Duration::from_secs"))]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub response_latency_s: Duration,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "client", non_exhaustive)]
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

impl AttemptDetails {
    pub fn status_message(&self) -> String {
        match self {
            | Self::WebhookAttemptDetails(details) => {
                format!(
                    "{}{}",
                    details
                        .response_code
                        .map(|a| format!("{} ", a))
                        .unwrap_or_default(),
                    details.error_message.as_deref().unwrap_or_default()
                )
            }
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::AttemptStatus")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Succeeded,
    Failed,
}

impl std::fmt::Display for AttemptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_variant::to_variant_name(self).unwrap())
    }
}
