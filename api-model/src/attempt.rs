use std::time::Duration;

use chrono::{DateTime, Utc};
#[cfg(feature = "dto")]
use dto::FromProto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};
use strum::Display;

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
    #[cfg_attr(feature = "dto", from_proto(map = "Duration::from_secs_f64"))]
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
#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::attempt_proto::AttemptStatus")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
// It's unfortunate that strum won't read serde's rename_all attribute. Maybe we
// can contribute by addressing the [issue](https://github.com/Peternator7/strum/issues/113)
#[strum(serialize_all = "snake_case")]
pub enum AttemptStatus {
    Succeeded,
    Failed,
}

#[cfg(test)]
mod test {
    use crate::AttemptStatus;

    #[test]
    fn attempt_status_to_string() {
        // swap the order of arguments of assert_eq in the next two lines
        assert_eq!("succeeded", AttemptStatus::Succeeded.to_string());
        assert_eq!("failed", AttemptStatus::Failed.to_string());
    }
}
