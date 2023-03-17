use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::Validate;

use shared::types::*;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct InvokeTrigger {
    pub id: TriggerId,
    // TODO: add support for webhook url overrides.
}

impl InvokeTrigger {
    pub fn from_id(id: TriggerId) -> Self {
        Self { id }
    }
}

impl From<InvokeTrigger> for scheduler_proto::InvokeTriggerRequest {
    fn from(value: InvokeTrigger) -> Self {
        scheduler_proto::InvokeTriggerRequest {
            id: value.id.into(),
        }
    }
}
