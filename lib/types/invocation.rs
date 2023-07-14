use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::{InvocationId, OwnerId, Payload, TriggerId, Webhook};
use crate::timeutil::iso8601_dateformat_serde;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub id: InvocationId,
    pub trigger_id: TriggerId,
    pub owner_id: OwnerId,
    #[serde(with = "iso8601_dateformat_serde")]
    pub created_at: DateTime<Tz>,
    pub payload: Payload,
    pub status: Vec<InvocationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum InvocationStatus {
    WebhookStatus(WebhookStatus),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct WebhookStatus {
    pub webhook: Webhook,
    pub delivery_status: WebhookDeliveryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum WebhookDeliveryStatus {
    Attempting,
    Succeeded,
    Failed,
}

impl Default for WebhookStatus {
    fn default() -> Self {
        Self {
            webhook: Default::default(),
            delivery_status: WebhookDeliveryStatus::Attempting,
        }
    }
}
