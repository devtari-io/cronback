use std::time::Duration;

use chrono::{DateTime, Utc};
use proto::{run_proto, trigger_proto, webhook_proto};

use super::{
    Action,
    ExponentialBackoffRetry,
    HttpMethod,
    Payload,
    Recurring,
    RetryConfig,
    Run,
    RunAt,
    RunStatus,
    Schedule,
    SimpleRetry,
    Status,
    Trigger,
    TriggerManifest,
    Webhook,
};
use crate::model::ValidShardedId;
use crate::timeutil::parse_iso8601_and_duration;

impl From<trigger_proto::Trigger> for Trigger {
    fn from(value: trigger_proto::Trigger) -> Self {
        Self {
            id: value.id.into(),
            project: ValidShardedId::from_string_unsafe(value.project_id),
            name: value.name,
            description: value.description,
            created_at: DateTime::parse_from_rfc3339(&value.created_at)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: value.updated_at.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
            reference: value.reference,
            payload: value.payload.map(|p| p.into()),
            schedule: value.schedule.map(|s| s.into()),
            action: value.action.unwrap().into(),
            status: value.status.into(),
            // We are not supposed to send this to other services, it is
            // internal.
            last_ran_at: value.last_ran_at.map(|l| {
                DateTime::parse_from_rfc3339(&l)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
        }
    }
}

impl From<trigger_proto::TriggerManifest> for TriggerManifest {
    fn from(value: trigger_proto::TriggerManifest) -> Self {
        Self {
            id: value.id.into(),
            project: ValidShardedId::from_string_unsafe(value.project_id),
            name: value.name,
            description: value.description,
            created_at: DateTime::parse_from_rfc3339(&value.created_at)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: value.updated_at.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
            action: value.action.unwrap().into(),
            reference: value.reference,
            schedule: value.schedule.map(|s| s.into()),
            status: value.status.into(),
            // We are not supposed to send this to other services, it is
            // internal.
            last_ran_at: value.last_ran_at.map(|l| {
                DateTime::parse_from_rfc3339(&l)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
        }
    }
}

impl From<webhook_proto::Webhook> for Webhook {
    fn from(webhook: webhook_proto::Webhook) -> Self {
        Self {
            _kind: Default::default(),
            http_method: webhook.http_method.into(),
            url: Some(webhook.url),
            timeout_s: Duration::from_secs_f64(webhook.timeout_s),
            retry: webhook.retry.map(|r| r.into()),
        }
    }
}

impl From<trigger_proto::Action> for Action {
    fn from(value: trigger_proto::Action) -> Self {
        match value.action.unwrap() {
            | trigger_proto::action::Action::Webhook(webhook) => {
                Self::Webhook(webhook.into())
            }
        }
    }
}

impl From<trigger_proto::Schedule> for Schedule {
    fn from(value: trigger_proto::Schedule) -> Self {
        match value.schedule.unwrap() {
            | trigger_proto::schedule::Schedule::Recurring(recurring) => {
                Self::Recurring(recurring.into())
            }
            | trigger_proto::schedule::Schedule::RunAt(run_at) => {
                Self::RunAt(run_at.into())
            }
        }
    }
}

impl From<trigger_proto::Recurring> for Recurring {
    fn from(value: trigger_proto::Recurring) -> Self {
        Self {
            cron: Some(value.cron),
            timezone: value.timezone,
            limit: value.limit,
            remaining: value.remaining,
        }
    }
}

impl From<trigger_proto::RunAt> for RunAt {
    fn from(value: trigger_proto::RunAt) -> Self {
        Self {
            timepoints: value
                .timepoints
                .into_iter()
                .map(|d| parse_iso8601_and_duration(&d).unwrap())
                .collect(),
            remaining: value.remaining,
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
            | trigger_proto::TriggerStatus::Scheduled => Self::Scheduled,
            | trigger_proto::TriggerStatus::Paused => Self::Paused,
            | trigger_proto::TriggerStatus::Cancelled => Self::Cancelled,
            | trigger_proto::TriggerStatus::OnDemand => Self::OnDemand,
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
            | webhook_proto::HttpMethod::Get => HttpMethod::Get,
            | webhook_proto::HttpMethod::Post => HttpMethod::Post,
            | webhook_proto::HttpMethod::Put => HttpMethod::Put,
            | webhook_proto::HttpMethod::Delete => HttpMethod::Delete,
            | webhook_proto::HttpMethod::Patch => HttpMethod::Patch,
            | webhook_proto::HttpMethod::Head => HttpMethod::Head,
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
            _kind: Default::default(),
            max_num_attempts: retry.max_num_attempts,
            delay_s: Duration::from_secs_f64(retry.delay_s),
        }
    }
}

impl From<webhook_proto::ExponentialBackoffRetry> for ExponentialBackoffRetry {
    fn from(retry: webhook_proto::ExponentialBackoffRetry) -> Self {
        Self {
            _kind: Default::default(),
            max_num_attempts: retry.max_num_attempts,
            delay_s: Duration::from_secs_f64(retry.delay_s),
            max_delay_s: Duration::from_secs_f64(retry.max_delay_s),
        }
    }
}

impl From<run_proto::Run> for Run {
    fn from(value: run_proto::Run) -> Self {
        Self {
            id: value.id.into(),
            trigger: value.trigger_id.into(),
            project: ValidShardedId::from_string_unsafe(value.project_id),
            created_at: parse_iso8601_and_duration(&value.created_at)
                .unwrap()
                .with_timezone(&Utc),
            payload: value.payload.map(|p| p.into()),
            action: value.action.unwrap().into(),
            status: value.status.into(),
        }
    }
}

impl From<i32> for RunStatus {
    fn from(value: i32) -> Self {
        let enum_value = run_proto::RunStatus::from_i32(value).unwrap();
        match enum_value {
            | run_proto::RunStatus::Unknown => {
                panic!("We should never see RunStatus::Unknown")
            }
            | run_proto::RunStatus::Attempting => Self::Attempting,
            | run_proto::RunStatus::Succeeded => Self::Succeeded,
            | run_proto::RunStatus::Failed => Self::Failed,
        }
    }
}
