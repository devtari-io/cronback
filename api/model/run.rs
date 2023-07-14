use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use lib::types::RunId;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::{Action, ActionAttemptLog, Payload};

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

#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct RunTrigger {
    pub mode: RunMode,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, FromProto, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[proto(target = "proto::run_proto::Run")]
pub struct Run {
    #[proto(required)]
    pub id: RunId,
    #[proto(required)]
    pub created_at: DateTime<Utc>,
    pub payload: Option<Payload>,
    #[proto(required)]
    pub action: Action,
    pub status: RunStatus,
}

#[derive(Debug, Serialize, FromProto, Clone)]
#[proto(target = "proto::dispatcher_proto::GetRunResponse")]
pub struct GetRunResponse {
    #[serde(flatten)]
    #[proto(required)]
    pub run: Run,
    pub latest_attempts: Vec<ActionAttemptLog>,
}
