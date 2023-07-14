use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use lib::types::{ProjectId, RunId, TriggerId};
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::{Action, Payload};

#[derive(IntoProto, Debug, Deserialize, Serialize, Clone, Default)]
#[proto(target = "scheduler_proto::RunMode")]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunMode {
    Sync,
    #[default]
    Async,
}

#[derive(Debug, FromProto, Clone, Serialize, Deserialize, PartialEq)]
#[proto(target = "proto::run_proto::RunStatus")]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Attempting,
    Succeeded,
    Failed,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct RunTrigger {
    pub mode: RunMode,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, FromProto, Serialize, Deserialize, PartialEq)]
#[proto(target = "proto::run_proto::Run")]
pub struct Run {
    pub id: RunId,
    #[proto(name = "trigger_id")]
    pub trigger: TriggerId,
    #[proto(name = "project_id")]
    pub project: ProjectId,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub created_at: DateTime<Utc>,
    pub payload: Option<Payload>,
    #[proto(required)]
    pub action: Action,
    pub status: RunStatus,
}
