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

#[cfg_attr(feature = "client", non_exhaustive)]
#[derive(Debug, Deserialize, PartialEq, Serialize, Clone, Copy, Default)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto),
    proto(target = "proto::scheduler_proto::RunMode")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Sync,
    #[default]
    Async,
}

impl std::fmt::Display for RunMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_variant::to_variant_name(self).unwrap())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "client", non_exhaustive)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::run_proto::RunStatus")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Attempting,
    Succeeded,
    Failed,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_variant::to_variant_name(self).unwrap())
    }
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
