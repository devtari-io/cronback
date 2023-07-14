use chrono::{DateTime, Utc};
#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
#[cfg(feature = "dto")]
use lib::types::RunId;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::{Action, ActionAttemptLog, Payload};
#[cfg(not(feature = "dto"))]
use crate::RunId;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto),
    proto(target = "proto::scheduler_proto::RunMode")
)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Sync,
    #[default]
    Async,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::run_proto::RunStatus")
)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Attempting,
    Succeeded,
    Failed,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(Default),
    serde(default),
    serde(deny_unknown_fields)
)]
pub struct RunTrigger {
    pub mode: RunMode,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::run_proto::Run")
)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields))]
pub struct Run {
    #[cfg_attr(feature = "dto", proto(required))]
    pub id: RunId,
    #[cfg_attr(feature = "dto", proto(required))]
    pub created_at: DateTime<Utc>,
    pub payload: Option<Payload>,
    #[cfg_attr(feature = "dto", proto(required))]
    pub action: Action,
    pub status: RunStatus,
}

#[derive(Debug, Serialize, Clone)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::dispatcher_proto::GetRunResponse")
)]
pub struct GetRunResponse {
    #[serde(flatten)]
    #[cfg_attr(feature = "dto", proto(required))]
    pub run: Run,
    pub latest_attempts: Vec<ActionAttemptLog>,
}
