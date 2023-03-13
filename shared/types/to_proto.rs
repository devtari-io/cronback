use proto::attempt_proto;
use proto::invocation_proto;
use proto::trigger_proto;
use proto::webhook_proto;

use crate::timeutil::to_iso8601;

use super::AttemptDetails;
use super::AttemptStatus;
use super::EmitAttemptLog;
use super::Invocation;
use super::InvocationStatus;
use super::RetryConfig;
use super::Webhook;
use super::WebhookDeliveryStatus;
use super::WebhookStatus;
use super::{
    Cron, Emit, HttpMethod, Payload, RunAt, Schedule, Status, Trigger,
};

impl From<Trigger> for trigger_proto::Trigger {
    fn from(value: Trigger) -> Self {
        Self {
            id: value.id.into(),
            owner_id: value.owner_id.into(),
            name: value.name,
            description: value.description,
            created_at: to_iso8601(&value.created_at),
            reference_id: value.reference_id,
            payload: Some(value.payload.into()),
            schedule: value.schedule.map(|s| s.into()),
            emit: value.emit.into_iter().map(|e| e.into()).collect(),
            status: value.status.into(),
            on_success: None,
            on_failure: None,
            /*
            on_success: todo!(),
            on_failure: todo!(),
            */
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
            timezone: value.cron_timezone,
            events_limit: value.cron_events_limit,
        }
    }
}

impl From<RunAt> for trigger_proto::RunAt {
    fn from(value: RunAt) -> Self {
        Self {
            run_at: value.run_at.into_iter().map(|d| to_iso8601(&d)).collect(),
        }
    }
}

impl From<Emit> for trigger_proto::Emit {
    fn from(value: Emit) -> Self {
        let emit = match value {
            | Emit::Webhook(webhook) => {
                trigger_proto::emit::Emit::Webhook(webhook.into())
            }
        };
        trigger_proto::Emit { emit: Some(emit) }
    }
}

impl From<Status> for i32 {
    fn from(value: Status) -> Self {
        let enum_value = match value {
            | Status::Active => trigger_proto::TriggerStatus::Active,
            | Status::Expired => trigger_proto::TriggerStatus::Expired,
            | Status::Canceled => trigger_proto::TriggerStatus::Canceled,
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
            trigger_id: value.trigger_id.into(),
            owner_id: value.owner_id.into(),
            created_at: to_iso8601(&value.created_at),
            payload: Some(value.payload.into()),
            status: value.status.into_iter().map(|s| s.into()).collect(),
        }
    }
}

// AttemptLog

impl From<EmitAttemptLog> for attempt_proto::EmitAttemptLog {
    fn from(value: EmitAttemptLog) -> Self {
        Self {
            id: value.id.into(),
            invocation_id: value.invocation_id.into(),
            trigger_id: value.trigger_id.into(),
            owner_id: value.owner_id.into(),
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
                        attempt_count: webhook_details.attempt_count,
                        response_code: webhook_details.response_code,
                        response_latency_s: webhook_details
                            .response_latency_s
                            .as_secs_f64(),
                        response_payload: Some(
                            webhook_details.response_payload.into(),
                        ),
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
