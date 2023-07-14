use proto::{attempt_proto, invocation_proto, trigger_proto, webhook_proto};

use super::{
    AttemptDetails,
    AttemptStatus,
    Cron,
    Emit,
    EmitAttemptLog,
    HttpMethod,
    Invocation,
    InvocationStatus,
    Payload,
    RetryConfig,
    RunAt,
    Schedule,
    Status,
    Trigger,
    TriggerManifest,
    Webhook,
    WebhookDeliveryStatus,
    WebhookStatus,
};
use crate::timeutil::to_iso8601;

impl From<Trigger> for trigger_proto::Trigger {
    fn from(value: Trigger) -> Self {
        Self {
            id: value.id.into(),
            project_id: value.project.into(),
            name: value.name,
            description: value.description,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.map(|s| s.to_rfc3339()),
            reference: value.reference,
            payload: value.payload.map(|p| p.into()),
            schedule: value.schedule.map(|s| s.into()),
            emit: value.emit.into_iter().map(|e| e.into()).collect(),
            status: value.status.into(),
            on_success: None,
            on_failure: None,
        }
    }
}

impl From<TriggerManifest> for trigger_proto::TriggerManifest {
    fn from(value: TriggerManifest) -> Self {
        Self {
            id: value.id.into(),
            project_id: value.project.into(),
            name: value.name,
            description: value.description,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.map(|s| s.to_rfc3339()),
            emit: value.emit.into_iter().map(|e| e.into()).collect(),
            reference: value.reference,
            schedule: value.schedule.map(|s| s.into()),
            status: value.status.into(),
            last_invoked_at: value.last_invoked_at.map(|d| d.to_rfc3339()),
        }
    }
}

impl From<Payload> for trigger_proto::Payload {
    fn from(value: Payload) -> Self {
        Self {
            content_type: value.content_type,
            headers: value.headers,
            body: value.body.into(),
        }
    }
}

impl From<Schedule> for trigger_proto::Schedule {
    fn from(value: Schedule) -> Self {
        let schedule = match value {
            | Schedule::Recurring(cron) => {
                trigger_proto::schedule::Schedule::Cron(cron.into())
            }
            | Schedule::RunAt(run_at) => {
                trigger_proto::schedule::Schedule::RunAt(run_at.into())
            }
        };
        Self {
            schedule: Some(schedule),
        }
    }
}

impl From<Cron> for trigger_proto::Cron {
    fn from(value: Cron) -> Self {
        Self {
            cron: value.cron.unwrap(),
            timezone: value.timezone,
            limit: value.limit,
            remaining: value.remaining,
        }
    }
}

impl From<RunAt> for trigger_proto::RunAt {
    fn from(value: RunAt) -> Self {
        Self {
            run_at: value
                .timepoints
                .into_iter()
                .map(|d| to_iso8601(&d))
                .collect(),
            remaining: value.remaining,
        }
    }
}

impl From<Emit> for trigger_proto::Emit {
    fn from(value: Emit) -> Self {
        let emit = match value {
            | Emit::Webhook(webhook) => {
                trigger_proto::emit::Emit::Webhook(webhook.into())
            }
            | Emit::Event(_) => unimplemented!(),
        };
        trigger_proto::Emit { emit: Some(emit) }
    }
}

impl From<Status> for i32 {
    fn from(value: Status) -> Self {
        let enum_value = match value {
            | Status::Scheduled => trigger_proto::TriggerStatus::Scheduled,
            | Status::Expired => trigger_proto::TriggerStatus::Expired,
            | Status::OnDemand => trigger_proto::TriggerStatus::OnDemand,
            | Status::Cancelled => trigger_proto::TriggerStatus::Cancelled,
            | Status::Paused => trigger_proto::TriggerStatus::Paused,
        };
        enum_value as i32
    }
}

impl From<HttpMethod> for i32 {
    fn from(value: HttpMethod) -> Self {
        let enum_value = match value {
            | HttpMethod::GET => webhook_proto::HttpMethod::Get,
            | HttpMethod::POST => webhook_proto::HttpMethod::Post,
            | HttpMethod::PUT => webhook_proto::HttpMethod::Put,
            | HttpMethod::DELETE => webhook_proto::HttpMethod::Delete,
            | HttpMethod::PATCH => webhook_proto::HttpMethod::Patch,
            | HttpMethod::HEAD => webhook_proto::HttpMethod::Head,
        };
        enum_value as i32
    }
}

impl From<Webhook> for webhook_proto::Webhook {
    fn from(value: Webhook) -> Self {
        Self {
            http_method: value.http_method.into(),
            url: value.url.unwrap(),
            timeout_s: value.timeout_s.as_secs_f64(),
            retry: value.retry.map(|r| r.into()),
        }
    }
}

impl From<RetryConfig> for webhook_proto::RetryConfig {
    fn from(value: RetryConfig) -> Self {
        let policy = match value {
            | RetryConfig::SimpleRetry(simple) => {
                webhook_proto::retry_config::Policy::Simple(
                    webhook_proto::SimpleRetry {
                        max_num_attempts: simple.max_num_attempts,
                        delay_s: simple.delay_s.as_secs_f64(),
                    },
                )
            }
            | RetryConfig::ExponentialBackoffRetry(exponential) => {
                webhook_proto::retry_config::Policy::ExponentialBackoff(
                    webhook_proto::ExponentialBackoffRetry {
                        max_num_attempts: exponential.max_num_attempts,
                        delay_s: exponential.delay_s.as_secs_f64(),
                        max_delay_s: exponential.max_delay_s.as_secs_f64(),
                    },
                )
            }
        };
        webhook_proto::RetryConfig {
            policy: Some(policy),
        }
    }
}

impl From<WebhookDeliveryStatus> for i32 {
    fn from(value: WebhookDeliveryStatus) -> Self {
        let enum_value: invocation_proto::WebhookDeliveryStatus = match value {
            | WebhookDeliveryStatus::Failed => {
                invocation_proto::WebhookDeliveryStatus::Failed
            }
            | WebhookDeliveryStatus::Attempting => {
                invocation_proto::WebhookDeliveryStatus::Attempting
            }
            | WebhookDeliveryStatus::Succeeded => {
                invocation_proto::WebhookDeliveryStatus::Succeeded
            }
        };
        enum_value as i32
    }
}

impl From<WebhookStatus> for invocation_proto::WebhookStatus {
    fn from(value: WebhookStatus) -> Self {
        Self {
            webhook: Some(value.webhook.into()),
            delivery_status: value.delivery_status.into(),
        }
    }
}

impl From<InvocationStatus> for invocation_proto::InvocationStatus {
    fn from(value: InvocationStatus) -> Self {
        let status = match value {
            | InvocationStatus::WebhookStatus(webhook) => {
                invocation_proto::invocation_status::Status::Webhook(
                    webhook.into(),
                )
            }
        };
        invocation_proto::InvocationStatus {
            status: Some(status),
        }
    }
}

impl From<Invocation> for invocation_proto::Invocation {
    fn from(value: Invocation) -> Self {
        Self {
            id: value.id.into(),
            trigger_id: value.trigger.into(),
            project_id: value.project.into(),
            created_at: to_iso8601(&value.created_at),
            payload: value.payload.map(|p| p.into()),
            status: value.status.into_iter().map(|s| s.into()).collect(),
        }
    }
}

// AttemptLog

impl From<EmitAttemptLog> for attempt_proto::EmitAttemptLog {
    fn from(value: EmitAttemptLog) -> Self {
        Self {
            id: value.id.into(),
            invocation_id: value.invocation.into(),
            trigger_id: value.trigger.into(),
            project_id: value.project.into(),
            status: value.status.into(),
            details: Some(value.details.into()),
            created_at: to_iso8601(&value.created_at),
        }
    }
}

impl From<AttemptDetails> for attempt_proto::AttemptDetails {
    fn from(value: AttemptDetails) -> Self {
        let details = match value {
            | AttemptDetails::WebhookAttemptDetails(webhook_details) => {
                attempt_proto::attempt_details::Details::WebhookDetails(
                    attempt_proto::WebhookAttemptDetails {
                        response_code: webhook_details.response_code,
                        response_latency_s: webhook_details
                            .response_latency_s
                            .as_secs_f64(),
                        error_msg: webhook_details.error_message,
                    },
                )
            }
        };
        attempt_proto::AttemptDetails {
            details: Some(details),
        }
    }
}

impl From<AttemptStatus> for i32 {
    fn from(value: AttemptStatus) -> Self {
        let enum_value: attempt_proto::AttemptStatus = match value {
            | AttemptStatus::Failed => attempt_proto::AttemptStatus::Failed,
            | AttemptStatus::Succeeded => {
                attempt_proto::AttemptStatus::Succeeded
            }
        };
        enum_value as i32
    }
}
