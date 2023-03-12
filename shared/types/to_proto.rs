use proto::trigger_proto;
use proto::webhook_proto;

use crate::timeutil::to_iso8601;

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
            emit: value.emit.map(|e| e.into()),
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
                trigger_proto::emit::Emit::Webhook(webhook_proto::Webhook {
                    http_method: webhook.http_method.into(),
                    url: webhook.url.unwrap(),
                    timeout_s: webhook.timeout_s.as_secs_f64(),
                })
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
