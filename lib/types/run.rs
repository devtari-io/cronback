use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::{Action, Payload, ProjectId, RunId, TriggerId};
use crate::model::ValidShardedId;
use crate::timeutil::iso8601_dateformat_serde;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Run {
    pub id: RunId,
    pub trigger: TriggerId,
    pub project: ValidShardedId<ProjectId>,
    #[serde(with = "iso8601_dateformat_serde")]
    pub created_at: DateTime<Tz>,
    pub payload: Option<Payload>,
    pub action: Action,
    pub status: RunStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Attempting,
    Succeeded,
    Failed,
}
