use lib::model::ValidShardedId;
use lib::types::*;
use names::Generator;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::Validate;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct InstallTrigger {
    #[validate(length(
        min = 2,
        max = 1000,
        message = "name must be between 2 and 1000 characters if set"
    ))]
    pub name: Option<String>,

    pub description: Option<String>,

    pub reference: Option<String>,

    pub payload: Option<Payload>,

    #[validate]
    pub schedule: Option<Schedule>,

    #[validate(length(
        min = 1,
        message = "emit must contain at least one emit"
    ))]
    // Necessary to perform nested validation.
    #[validate]
    #[serde_as(
        as = "serde_with::OneOrMany<_, serde_with::formats::PreferMany>"
    )]
    pub emit: Vec<Emit>,
}

impl InstallTrigger {
    pub fn into_proto(
        self,
        project: ValidShardedId<ProjectId>,
        id: Option<ValidShardedId<TriggerId>>,
        fail_if_exists: bool,
    ) -> scheduler_proto::InstallTriggerRequest {
        let mut generator = Generator::default();
        scheduler_proto::InstallTriggerRequest {
            id: id.map(Into::into),
            fail_if_exists,
            project_id: project.into(),
            name: self.name.unwrap_or_else(|| generator.next().unwrap()),
            description: self.description,
            reference: self.reference,
            payload: self.payload.map(|p| p.into()),
            emit: self.emit.into_iter().map(|e| e.into()).collect(),
            schedule: self.schedule.map(|s| s.into()),
        }
    }
}
