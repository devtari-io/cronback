use std::time::Duration;

use proto::trigger_proto;
use proto::webhook_proto;

use crate::timeutil::parse_iso8601;

use super::{Emit, HttpMethod, Payload, Schedule, Status, Trigger};

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
            emit: value.emit.map(|e| e.into()),
            status: value.status.into(),
        }
    }
}

impl From<trigger_proto::Emit> for Emit {
    fn from(value: trigger_proto::Emit) -> Self {
        match value.emit.unwrap() {
            | trigger_proto::emit::Emit::Webhook(webhook) => {
                Self::Webhook(super::Webhook {
                    http_method: webhook.http_method.into(),
                    url: Some(webhook.url),
                    timeout_s: Duration::from_secs_f64(webhook.timeout_s),
                })
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
