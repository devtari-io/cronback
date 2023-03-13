use std::time::Duration;

use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_with::DurationSecondsWithFrac;
use serde_with::{serde_as, skip_serializing_none};

use crate::timeutil::iso8601_dateformat_serde;

use super::{AttemptLogId, InvocationId, OwnerId, Payload, TriggerId};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmitAttemptLog {
    pub id: AttemptLogId,
    pub invocation_id: InvocationId,
    pub trigger_id: TriggerId,
    pub owner_id: OwnerId,
    pub status: AttemptStatus,
    pub details: AttemptDetails,
    #[serde(with = "iso8601_dateformat_serde")]
    pub created_at: DateTime<Tz>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WebhookAttemptDetails {
    pub attempt_count: u32,
    pub response_code: i32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub response_latency_s: Duration,
    pub response_payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AttemptDetails {
    WebhookAttemptDetails(WebhookAttemptDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum AttemptStatus {
    Succeeded,
    Failed,
}
