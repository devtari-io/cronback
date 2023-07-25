use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use cronback_api_model::Trigger;
use proto::common::{RequestPrecondition, UpsertEffect};
use proto::scheduler_svc;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use tracing::error;
use validator::Validate;

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
    ) -> scheduler_svc::UpsertTriggerRequest {
        scheduler_svc::UpsertTriggerRequest {
            precondition,
            trigger_name,
            trigger: Some(self.trigger.into()),
        }
    }
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
