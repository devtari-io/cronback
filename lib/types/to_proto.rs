use proto::{run_proto, trigger_proto, webhook_proto};

use super::{
    Action,
    HttpMethod,
    Payload,
    Recurring,
    RetryConfig,
    Run,
    RunAt,
    RunStatus,
    Schedule,
    Status,
    Trigger,
    TriggerManifest,
    Webhook,
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
            action: Some(value.action.into()),
            status: value.status.into(),
            last_ran_at: value.last_ran_at.map(|d| d.to_rfc3339()),
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
            action: Some(value.action.into()),
            reference: value.reference,
            schedule: value.schedule.map(|s| s.into()),
            status: value.status.into(),
            last_ran_at: value.last_ran_at.map(|d| d.to_rfc3339()),
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
            | Schedule::Recurring(recurring) => {
                trigger_proto::schedule::Schedule::Recurring(recurring.into())
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

impl From<Recurring> for trigger_proto::Recurring {
    fn from(value: Recurring) -> Self {
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
            timepoints: value
                .timepoints
                .into_iter()
                .map(|d| to_iso8601(&d))
                .collect(),
            remaining: value.remaining,
        }
    }
}

impl From<Action> for trigger_proto::Action {
    fn from(value: Action) -> Self {
        let action = match value {
            | Action::Webhook(webhook) => {
                trigger_proto::action::Action::Webhook(webhook.into())
            }
            | Action::Event(_) => unimplemented!(),
        };
        trigger_proto::Action {
            action: Some(action),
        }
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
            | HttpMethod::Get => webhook_proto::HttpMethod::Get,
            | HttpMethod::Post => webhook_proto::HttpMethod::Post,
            | HttpMethod::Put => webhook_proto::HttpMethod::Put,
            | HttpMethod::Delete => webhook_proto::HttpMethod::Delete,
            | HttpMethod::Patch => webhook_proto::HttpMethod::Patch,
            | HttpMethod::Head => webhook_proto::HttpMethod::Head,
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

impl From<RunStatus> for i32 {
    fn from(value: RunStatus) -> Self {
        let enum_value: run_proto::RunStatus = match value {
            | RunStatus::Failed => run_proto::RunStatus::Failed,
            | RunStatus::Attempting => run_proto::RunStatus::Attempting,
            | RunStatus::Succeeded => run_proto::RunStatus::Succeeded,
        };
        enum_value as i32
    }
}

impl From<Run> for run_proto::Run {
    fn from(value: Run) -> Self {
        Self {
            id: value.id.into(),
            trigger_id: value.trigger.into(),
            project_id: value.project.into(),
            created_at: to_iso8601(&value.created_at),
            payload: value.payload.map(|p| p.into()),
            action: Some(value.action.into()),
            status: value.status.into(),
        }
    }
}
