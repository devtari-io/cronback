use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use lib::model::ValidShardedId;
use lib::types::{ProjectId, Trigger, TriggerId};
use names::Generator;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::Validate;

use super::{Emit, Payload, Schedule};

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

impl InstallTriggerRequest {
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
            payload: self.payload.map(Into::into),
            emit: self.emit.into_iter().map(Into::into).collect(),
            schedule: self.schedule.map(Into::into),
        }
    }
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
        // List of emits.
        std::env::set_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION", "true");
        // Validate that the following JSON install trigger can be parsed
        // correctly.

        let request = json!(
            {
              "name": "test-trigger-1",
              "schedule": {
                "timepoints": [ "PT1M", "PT2M" ]
              },
              "payload": {
                "content_type": "application/json",
                "headers": {
                  "X-My-Header": "my-header-value"
                },
                "body": "some-body"
              },
              // Emit is a list.
              "emit": [
                {
                  "type": "webhook",
                  "url": "http://localhost:3000/emit",
                  "timeout_s": 30
                }
              ]
            }
        );

        //
        let parsed: InstallTriggerRequest = serde_json::from_value(request)?;

        assert_eq!(parsed.name, Some("test-trigger-1".to_string()));
        assert!(parsed.schedule.is_some());
        parsed.validate()?;
        Ok(())
        // TODO: Switch to using insta for snapshot testing.
    }

    #[test]
    fn validate_install_trigger_02() -> Result<()> {
        // Single emit. no payload.
        std::env::set_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION", "true");

        let request = json!(
          {
            "schedule": {
              "cron": "*/3 * * * * *",
              "limit": 5
            },
            "emit": {
              "url": "http://localhost:3000/emit",
              "timeout_s": 10,
              "retry": {
                "delay_s": 100
              }

            }
          }
        );

        //
        let parsed: InstallTriggerRequest = serde_json::from_value(request)?;
        parsed.validate()?;
        Ok(())
    }
}
