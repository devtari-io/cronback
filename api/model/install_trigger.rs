use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use lib::model::ValidShardedId;
use lib::types::{ProjectId, TriggerId};
use names::Generator;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::Validate;

use super::{Action, Payload, Schedule};

#[derive(
    Debug,
    IntoProto,
    FromProto,
    Default,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
)]
#[proto(target = "proto::trigger_proto::TriggerStatus")]
#[serde(rename_all = "snake_case")]
pub enum TriggerStatus {
    #[default]
    Scheduled,
    OnDemand,
    Expired,
    Cancelled,
    Paused,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallTriggerRequest {
    #[validate(length(
        min = 2,
        max = 1000,
        message = "name must be between 2 and 1000 characters if set"
    ))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub reference: Option<String>,
    #[validate]
    pub payload: Option<Payload>,
    #[validate]
    pub schedule: Option<Schedule>,
    #[validate]
    pub action: Action,
}

impl InstallTriggerRequest {
    pub fn into_proto(
        self,
        id: Option<ValidShardedId<TriggerId>>,
        fail_if_exists: bool,
    ) -> scheduler_proto::InstallTriggerRequest {
        let mut generator = Generator::default();
        scheduler_proto::InstallTriggerRequest {
            id: id.map(Into::into),
            fail_if_exists,
            name: self.name.unwrap_or_else(|| generator.next().unwrap()),
            description: self.description,
            reference: self.reference,
            payload: self.payload.map(Into::into),
            action: Some(self.action.into()),
            schedule: self.schedule.map(Into::into),
        }
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, FromProto, Clone, Serialize, Deserialize, PartialEq)]
#[proto(target = "proto::trigger_proto::Trigger")]
pub(crate) struct Trigger {
    pub id: TriggerId,
    pub name: String,
    pub description: Option<String>,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub created_at: DateTime<Utc>,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub updated_at: Option<DateTime<Utc>>,
    pub reference: Option<String>,
    pub payload: Option<Payload>,
    pub schedule: Option<Schedule>,
    #[proto(required)]
    pub action: Action,
    pub status: TriggerStatus,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub last_ran_at: Option<DateTime<Utc>>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, FromProto, Clone, Serialize, Deserialize, PartialEq)]
#[proto(target = "proto::trigger_proto::TriggerManifest")]
pub(crate) struct TriggerManifest {
    pub id: TriggerId,
    #[proto(name = "project_id")]
    pub project: ProjectId,
    pub name: String,
    pub description: Option<String>,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub created_at: DateTime<Utc>,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub updated_at: Option<DateTime<Utc>>,
    #[proto(required)]
    pub action: Action,
    pub reference: Option<String>,
    pub schedule: Option<Schedule>,
    pub status: TriggerStatus,
    #[proto(
        map_from_proto = "lib::timeutil::parse_utc_from_rfc3339",
        map_from_by_ref
    )]
    pub last_ran_at: Option<DateTime<Utc>>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InstallTriggerResponse {
    #[serde(flatten)]
    pub trigger: Trigger,
    #[serde(skip_serializing)]
    pub already_existed: bool,
}

impl IntoResponse for InstallTriggerResponse {
    fn into_response(self) -> axum::response::Response {
        let status = if self.already_existed {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        };

        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::*;

    #[test]
    fn validate_install_trigger_01() -> Result<()> {
        std::env::set_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION", "true");

        let request = json!(
          {
            "schedule": {
              "cron": "*/3 * * * * *",
              "limit": 5
            },
            "action": {
              "url": "http://localhost:3000/action",
              "timeout_s": 10,
              "retry": {
                "delay_s": 100
              }

            }
          }
        );

        let parsed: InstallTriggerRequest = serde_json::from_value(request)?;
        parsed.validate()?;
        Ok(())
    }
}
