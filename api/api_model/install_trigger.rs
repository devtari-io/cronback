use lib::types::*;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::Validate;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct InstallTrigger {
    #[validate(length(
        min = 2,
        max = 1000,
        message = "name must be between 2 and 1000 characters if set"
    ))]
    pub name: Option<String>,

    pub description: Option<String>,

    pub reference_id: Option<String>,

    pub payload: Payload,

    #[validate]
    pub schedule: Option<Schedule>,

    #[validate(length(
        min = 1,
        message = "emit must contain at least one emit"
    ))]
    // Necessary to perform nested validation.
    #[validate]
    pub emit: Vec<Emit>,
}

impl InstallTrigger {
    pub fn into_proto(
        self,
        owner_id: OwnerId,
        cell_id: CellId,
    ) -> scheduler_proto::InstallTriggerRequest {
        scheduler_proto::InstallTriggerRequest {
            owner_id: owner_id.into(),
            cell_id: cell_id.into(),
            name: self.name,
            description: self.description,
            reference_id: self.reference_id,
            payload: Some(self.payload.into()),
            emit: self.emit.into_iter().map(|e| e.into()).collect(),
            schedule: self.schedule.map(|s| s.into()),
        }
    }
}
