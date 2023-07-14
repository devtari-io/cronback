use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::{InvocationId, Payload, ProjectId, TriggerId, Webhook};
use crate::model::ValidShardedId;
use crate::timeutil::iso8601_dateformat_serde;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invocation {
    pub id: InvocationId,
    pub trigger: TriggerId,
    pub project: ValidShardedId<ProjectId>,
    #[serde(with = "iso8601_dateformat_serde")]
    pub created_at: DateTime<Tz>,
    pub payload: Option<Payload>,
    pub status: Vec<InvocationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum InvocationStatus {
    WebhookStatus(WebhookStatus),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookStatus {
    #[serde(flatten)]
    pub webhook: Webhook,
    pub delivery_status: WebhookDeliveryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookDeliveryStatus {
    Attempting,
    Succeeded,
    Failed,
}
