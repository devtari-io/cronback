use std::time::Duration;

use super::AttemptDetails;
use super::AttemptStatus;
use super::EmitAttemptLog;
use super::ExponentialBackoffRetry;
use super::RetryConfig;
use super::SimpleRetry;
use super::Webhook;
use super::WebhookAttemptDetails;
use super::{Emit, HttpMethod, Payload, Schedule, Status, Trigger};
use crate::timeutil::parse_iso8601;
use proto::attempt_proto;
use proto::trigger_proto;
use proto::webhook_proto;

impl From<trigger_proto::Trigger> for Trigger {
    fn from(value: trigger_proto::Trigger) -> Self {
        Self {
            id: value.id.into(),
            owner_id: value.owner_id.into(),
            name: value.name,
            description: value.description,
            created_at: parse_iso8601(&value.created_at).unwrap(),
            reference_id: value.reference_id,
            payload: value.payload.unwrap().into(),
            schedule: value.schedule.map(|s| s.into()),
            emit: value.emit.into_iter().map(|e| e.into()).collect(),
            status: value.status.into(),
        }
    }
}

impl From<webhook_proto::Webhook> for Webhook {
    fn from(webhook: webhook_proto::Webhook) -> Self {
        Self {
            http_method: webhook.http_method.into(),
            url: Some(webhook.url),
            timeout_s: Duration::from_secs_f64(webhook.timeout_s),
            retry: webhook.retry.map(|r| r.into()),
        }
    }
}

impl From<trigger_proto::Emit> for Emit {
    fn from(value: trigger_proto::Emit) -> Self {
        match value.emit.unwrap() {
            | trigger_proto::emit::Emit::Webhook(webhook) => {
                Self::Webhook(webhook.into())
            }
        }
    }
}

impl From<trigger_proto::Schedule> for Schedule {
    fn from(value: trigger_proto::Schedule) -> Self {
        match value.schedule.unwrap() {
            | trigger_proto::schedule::Schedule::Cron(cron) => {
                Self::Recurring(cron.into())
            }
            | trigger_proto::schedule::Schedule::RunAt(run_at) => {
                Self::RunAt(run_at.into())
            }
        }
    }
}

impl From<trigger_proto::Cron> for super::Cron {
    fn from(value: trigger_proto::Cron) -> Self {
        Self {
            cron: Some(value.cron),
            cron_timezone: value.timezone,
            cron_events_limit: value.events_limit,
        }
    }
}

impl From<trigger_proto::RunAt> for super::RunAt {
    fn from(value: trigger_proto::RunAt) -> Self {
        Self {
            run_at: value
                .run_at
                .into_iter()
                .map(|d| parse_iso8601(&d).unwrap())
                .collect(),
        }
    }
}

impl From<trigger_proto::Payload> for Payload {
    fn from(value: trigger_proto::Payload) -> Self {
        Self {
            content_type: value.content_type,
            headers: value.headers,
            body: String::from_utf8(value.body).unwrap(),
        }
    }
}

impl From<i32> for Status {
    fn from(value: i32) -> Self {
        let enum_value = trigger_proto::TriggerStatus::from_i32(value).unwrap();
        match enum_value {
            | trigger_proto::TriggerStatus::Unknown => {
                panic!("We should never see TriggerStatus::Unknown")
            }
            | trigger_proto::TriggerStatus::Active => Self::Active,
            | trigger_proto::TriggerStatus::Paused => Self::Paused,
            | trigger_proto::TriggerStatus::Canceled => Self::Canceled,
            | trigger_proto::TriggerStatus::Expired => Self::Expired,
        }
    }
}

impl From<i32> for HttpMethod {
    fn from(value: i32) -> Self {
        let enum_value = webhook_proto::HttpMethod::from_i32(value).unwrap();
        match enum_value {
            | webhook_proto::HttpMethod::Unknown => {
                panic!("We should never see HttpMethod::Unknown")
            }
            | webhook_proto::HttpMethod::Get => HttpMethod::GET,
            | webhook_proto::HttpMethod::Post => HttpMethod::POST,
            | webhook_proto::HttpMethod::Put => HttpMethod::PUT,
            | webhook_proto::HttpMethod::Delete => HttpMethod::DELETE,
            | webhook_proto::HttpMethod::Patch => HttpMethod::PATCH,
            | webhook_proto::HttpMethod::Head => HttpMethod::HEAD,
        }
    }
}

impl From<webhook_proto::RetryConfig> for RetryConfig {
    fn from(value: webhook_proto::RetryConfig) -> Self {
        match value.policy.unwrap() {
            | webhook_proto::retry_config::Policy::Simple(simple) => {
                Self::SimpleRetry(simple.into())
            }
            | webhook_proto::retry_config::Policy::ExponentialBackoff(
                exponential,
            ) => Self::ExponentialBackoffRetry(exponential.into()),
        }
    }
}

impl From<webhook_proto::SimpleRetry> for SimpleRetry {
    fn from(retry: webhook_proto::SimpleRetry) -> Self {
        Self {
            max_num_attempts: retry.max_num_attempts,
            delay_s: Duration::from_secs_f64(retry.delay_s),
        }
    }
}

impl From<webhook_proto::ExponentialBackoffRetry> for ExponentialBackoffRetry {
    fn from(retry: webhook_proto::ExponentialBackoffRetry) -> Self {
        Self {
            max_num_attempts: retry.max_num_attempts,
            delay_s: Duration::from_secs_f64(retry.delay_s),
            max_delay_s: Duration::from_secs_f64(retry.max_delay_s),
        }
    }
}

// AttemptLog

impl From<i32> for AttemptStatus {
    fn from(value: i32) -> Self {
        let enum_value = attempt_proto::AttemptStatus::from_i32(value).unwrap();
        match enum_value {
            | attempt_proto::AttemptStatus::Unknown => {
                panic!("We should never see AttemptStatus::Unknown")
            }
            | attempt_proto::AttemptStatus::Failed => AttemptStatus::Failed,
            | attempt_proto::AttemptStatus::Succeeded => {
                AttemptStatus::Succeeded
            }
        }
    }
}

impl From<attempt_proto::EmitAttemptLog> for EmitAttemptLog {
    fn from(value: attempt_proto::EmitAttemptLog) -> Self {
        Self {
            id: value.id.into(),
            invocation_id: value.invocation_id.into(),
            trigger_id: value.trigger_id.into(),
            owner_id: value.owner_id.into(),
            status: value.status.into(),
            details: value.details.unwrap().into(),
        }
    }
}

impl From<attempt_proto::WebhookAttemptDetails> for WebhookAttemptDetails {
    fn from(value: attempt_proto::WebhookAttemptDetails) -> Self {
        Self {
            attempt_count: value.attempt_count,
            response_code: value.response_code,
            response_latency_s: Duration::from_secs_f64(
                value.response_latency_s,
            ),
            response_payload: value.response_payload.unwrap().into(),
        }
    }
}

impl From<attempt_proto::AttemptDetails> for AttemptDetails {
    fn from(value: attempt_proto::AttemptDetails) -> Self {
        match value.details.unwrap() {
            | attempt_proto::attempt_details::Details::WebhookDetails(
                details,
            ) => Self::WebhookAttemptDetails(details.into()),
        }
    }
}
