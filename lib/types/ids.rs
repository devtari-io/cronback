use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};

use super::ShardedId;
use crate::model::{
    generate_model_id,
    generate_project_id,
    shard_from_raw_project_id,
    ValidId,
};

#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct ProjectId(pub String);

impl ProjectId {
    pub fn new() -> Self {
        Self(generate_project_id("prj"))
    }

    pub fn from(value: String) -> Self {
        Self(value)
    }
}

impl ValidId for ProjectId {
    fn is_valid(&self) -> bool {
        self.0.starts_with("prj_")
    }
}

impl ShardedId for ProjectId {
    // Project ids are special, we don't prefix the value with the shard Id,
    // instead, the shard Id is calculated from the raw value.
    fn shard(&self) -> crate::model::Shard {
        shard_from_raw_project_id(&self.0)
    }

    fn value(&self) -> &str {
        &self.0
    }
}

#[derive(
    Debug,
    Hash,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct TriggerId(pub String);
impl TriggerId {
    pub fn new(project: &ProjectId) -> Self {
        Self(generate_model_id("trig", project))
    }

    pub fn from(value: String) -> Self {
        Self(value)
    }
}

impl ValidId for TriggerId {
    fn is_valid(&self) -> bool {
        self.0.starts_with("trig_")
    }
}

impl ShardedId for TriggerId {
    fn value(&self) -> &str {
        &self.0
    }
}

#[derive(
    Debug,
    Hash,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct InvocationId(pub String);
impl InvocationId {
    pub fn new(project: &ProjectId) -> Self {
        Self(generate_model_id("inv", project))
    }
}

impl ValidId for InvocationId {
    fn is_valid(&self) -> bool {
        self.0.starts_with("inv_")
    }
}

impl ShardedId for InvocationId {
    fn value(&self) -> &str {
        &self.0
    }
}

#[derive(
    Debug,
    Hash,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct AttemptLogId(pub String);
impl AttemptLogId {
    pub fn new(project: &ProjectId) -> Self {
        Self(generate_model_id("att", project))
    }

    pub fn from(value: String) -> Self {
        Self(value)
    }
}

impl ValidId for AttemptLogId {
    fn is_valid(&self) -> bool {
        self.0.starts_with("att")
    }
}

impl ShardedId for AttemptLogId {
    fn value(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_sharding() {
        let project = ProjectId::new();
        let project_shard = project.shard();

        let trigger = TriggerId::new(&project);
        assert_eq!(trigger.shard(), project_shard);
    }
}
