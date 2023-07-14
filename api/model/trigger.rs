use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use dto::{FromProto, IntoProto};
use lib::types::TriggerId;
use proto::common::{RequestPrecondition, UpsertEffect};
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use tracing::error;
use validator::Validate;

use super::{Action, Payload, Schedule};

#[derive(Debug, IntoProto, Deserialize, Default, Validate)]
#[proto(target = "proto::scheduler_proto::ListTriggersFilter")]
pub(crate) struct ListFilters {
    #[serde(default)]
    #[proto(name = "statuses")]
    pub status: Vec<TriggerStatus>,
}

#[derive(
    Debug, IntoProto, FromProto, Clone, Serialize, Deserialize, PartialEq,
)]
#[proto(target = "proto::trigger_proto::TriggerStatus")]
#[serde(rename_all = "snake_case")]
pub enum TriggerStatus {
    Scheduled,
    OnDemand,
    Expired,
    Cancelled,
    Paused,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Validate)]
pub(crate) struct UpsertTriggerRequest {
    #[serde(flatten)]
    #[validate]
    pub trigger: Trigger,
}

impl UpsertTriggerRequest {
    pub fn into_proto(
        self,
        trigger_name: Option<String>,
        precondition: Option<RequestPrecondition>,
    ) -> scheduler_proto::UpsertTriggerRequest {
        scheduler_proto::UpsertTriggerRequest {
            precondition,
            trigger_name,
            trigger: Some(self.trigger.into()),
        }
    }
}

#[skip_serializing_none]
#[derive(
    Debug,
    FromProto,
    IntoProto,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Validate,
)]
#[serde(deny_unknown_fields)]
#[proto(target = "proto::trigger_proto::Trigger")]
pub(crate) struct Trigger {
    // Ids are meant to be internal only, so they are neither accepted as input
    // or outputed in the API. This is here just for IntoProto to work
    #[serde(skip)]
    pub id: Option<TriggerId>,
    #[validate(length(
        min = 2,
        max = 64,
        message = "name must be between 2 and 64 characters if set"
    ))]
    #[proto(required)]
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[validate]
    pub action: Option<Action>,
    #[validate]
    pub schedule: Option<Schedule>,
    pub status: Option<TriggerStatus>,
    pub last_ran_at: Option<DateTime<Utc>>,
    #[validate]
    pub payload: Option<Payload>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq, Validate)]
#[serde_as]
pub(crate) struct UpsertTriggerResponse {
    #[serde(flatten)]
    pub trigger: Trigger,
    #[serde(skip)]
    pub effect: UpsertEffect,
}

impl IntoResponse for UpsertTriggerResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.effect {
            | UpsertEffect::Created => StatusCode::CREATED,
            | UpsertEffect::Modified => StatusCode::OK,
            | UpsertEffect::NotModified => StatusCode::NOT_MODIFIED,
            | _ => {
                error!(
                    "We don't know how to handle upsert effect {:?}, \
                     returning 500",
                    self.effect
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
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
              "type": "recurring",
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

        let parsed: UpsertTriggerRequest = serde_json::from_value(request)?;
        parsed.validate()?;
        Ok(())
    }
}
