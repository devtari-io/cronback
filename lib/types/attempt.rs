use std::time::Duration;

use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};

use super::{AttemptLogId, InvocationId, ProjectId, TriggerId};
use crate::model::ValidShardedId;
use crate::timeutil::iso8601_dateformat_serde;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmitAttemptLog {
    pub id: AttemptLogId,
    pub invocation: InvocationId,
    pub trigger: TriggerId,
    pub project: ValidShardedId<ProjectId>,
    pub status: AttemptStatus,
    pub details: AttemptDetails,
    #[serde(with = "iso8601_dateformat_serde")]
    pub created_at: DateTime<Tz>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookAttemptDetails {
    pub response_code: Option<i32>,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub response_latency_s: Duration,
    pub error_message: Option<String>,
}

impl WebhookAttemptDetails {
    pub fn is_success(&self) -> bool {
        (200..=299).contains(&self.response_code.unwrap_or(500))
    }

    pub fn with_error(err: String) -> Self {
        Self {
            response_code: None,
            response_latency_s: Duration::default(),
            error_message: Some(err),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum AttemptDetails {
    WebhookAttemptDetails(WebhookAttemptDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Succeeded,
    Failed,
}
