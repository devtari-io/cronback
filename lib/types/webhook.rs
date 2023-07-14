use std::time::Duration;

use dto::{FromProto, IntoProto};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(
    Debug, FromProto, IntoProto, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[proto(target = "proto::webhook_proto::HttpMethod")]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Delete,
    Get,
    Head,
    Patch,
    Post,
    Put,
}

#[derive(
    Debug,
    FromProto,
    IntoProto,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Validate,
    Eq,
)]
#[proto(target = "proto::webhook_proto::Webhook")]
pub struct Webhook {
    pub url: String,
    pub http_method: HttpMethod,
    #[proto(map_from_proto = "Duration::from_secs_f64")]
    #[proto(
        map_into_proto = "std::time::Duration::as_secs_f64",
        map_into_by_ref
    )]
    pub timeout_s: std::time::Duration,
    // None means no retry
    pub retry: Option<RetryConfig>,
}

#[derive(
    Debug, IntoProto, FromProto, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[proto(target = "proto::webhook_proto::RetryConfig", oneof = "policy")]
pub enum RetryConfig {
    #[proto(name = "Simple")]
    SimpleRetry(SimpleRetry),
    #[proto(name = "ExponentialBackoff")]
    ExponentialBackoffRetry(ExponentialBackoffRetry),
}

#[derive(
    Debug, IntoProto, FromProto, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[proto(target = "proto::webhook_proto::SimpleRetry")]
pub struct SimpleRetry {
    pub max_num_attempts: u32,
    #[proto(
        map_into_proto = "std::time::Duration::as_secs_f64",
        map_into_by_ref
    )]
    #[proto(map_from_proto = "Duration::from_secs_f64")]
    pub delay_s: Duration,
}

#[derive(
    Debug, FromProto, IntoProto, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[proto(target = "proto::webhook_proto::ExponentialBackoffRetry")]
pub struct ExponentialBackoffRetry {
    pub max_num_attempts: u32,
    #[proto(
        map_into_proto = "std::time::Duration::as_secs_f64",
        map_into_by_ref
    )]
    #[proto(map_from_proto = "Duration::from_secs_f64")]
    pub delay_s: Duration,
    #[proto(
        map_into_proto = "std::time::Duration::as_secs_f64",
        map_into_by_ref
    )]
    #[proto(map_from_proto = "Duration::from_secs_f64")]
    pub max_delay_s: Duration,
}
