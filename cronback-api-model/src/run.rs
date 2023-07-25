use chrono::{DateTime, Utc};
#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
#[cfg(feature = "dto")]
use lib::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use strum::Display;

use super::{Action, Attempt, Payload};
#[cfg(not(feature = "dto"))]
use crate::RunId;

#[cfg_attr(feature = "client", non_exhaustive)]
#[derive(
    Debug, Display, Deserialize, PartialEq, Serialize, Clone, Copy, Default,
)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto),
    proto(target = "proto::scheduler_svc::RunMode")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RunMode {
    Sync,
    #[default]
    Async,
}

#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "client", non_exhaustive)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::runs::RunStatus")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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
    proto(target = "proto::runs::Run")
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
    pub latest_attempt: Option<Attempt>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(
    feature = "dto",
    derive(FromProto),
    proto(target = "proto::dispatcher_svc::GetRunResponse")
)]
pub struct GetRunResponse {
    #[serde(flatten)]
    #[cfg_attr(feature = "dto", proto(required))]
    pub run: Run,
}

#[cfg(test)]
mod test {
    use super::{RunMode, RunStatus};

    #[test]
    fn run_mode_to_string() {
        assert_eq!(RunMode::Sync.to_string(), "sync");
        assert_eq!(RunMode::Async.to_string(), "async");
    }

    #[test]
    fn run_status_to_string() {
        assert_eq!(RunStatus::Attempting.to_string(), "attempting");
        assert_eq!(RunStatus::Succeeded.to_string(), "succeeded");
        assert_eq!(RunStatus::Failed.to_string(), "failed");
    }
}
