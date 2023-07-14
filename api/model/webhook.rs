use std::time::Duration;

use dto_helpers::IntoProto;
use lib::validation::{validate_webhook_url, validation_error};
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use validator::{Validate, ValidationError};

#[derive(IntoProto, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[into_proto(into = "proto::webhook_proto::HttpMethod")]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
pub enum HttpMethod {
    Delete,
    Get,
    Head,
    Patch,
    Post,
    Put,
}

#[serde_as]
#[derive(
    IntoProto, Debug, Clone, Serialize, Deserialize, Validate, PartialEq,
)]
#[into_proto(into = "proto::webhook_proto::Webhook")]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    // allows an optional "type" field to be passed in. This enables other
    // variants of emit to be differentiated.
    #[serde(rename = "type")]
    _kind: MustBe!("webhook"),
    #[validate(required)]
    #[into_proto(required)]
    #[validate(required, custom = "validate_webhook_url")]
    pub url: Option<String>,
    pub http_method: HttpMethod,
    #[validate(custom = "validate_timeout")]
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[into_proto(map_method = "as_secs_f64")]
    pub timeout_s: std::time::Duration,
    // None means no retry
    pub retry: Option<RetryConfig>,
}

impl Default for Webhook {
    fn default() -> Self {
        Self {
            _kind: Default::default(),
            url: None,
            http_method: HttpMethod::Post,
            timeout_s: Duration::from_secs(5),
            retry: None,
        }
    }
}

#[derive(IntoProto, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[into_proto(into = "proto::webhook_proto::RetryConfig", oneof = "policy")]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum RetryConfig {
    #[into_proto(into = "Simple")]
    SimpleRetry(SimpleRetry),
    #[into_proto(into = "ExponentialBackoff")]
    ExponentialBackoffRetry(ExponentialBackoffRetry),
}

#[serde_as]
#[derive(
    IntoProto, Debug, Clone, Serialize, Deserialize, Validate, PartialEq,
)]
#[into_proto(into = "proto::webhook_proto::SimpleRetry")]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct SimpleRetry {
    #[serde(rename = "type")]
    _kind: MustBe!("simple"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[into_proto(map_method = "as_secs_f64")]
    pub delay_s: Duration,
}

impl Default for SimpleRetry {
    fn default() -> Self {
        Self {
            _kind: Default::default(),
            max_num_attempts: 5,
            delay_s: Duration::from_secs(60),
        }
    }
}

#[serde_as]
#[derive(
    IntoProto, Debug, Clone, Serialize, Deserialize, Validate, PartialEq,
)]
#[into_proto(into = "proto::webhook_proto::ExponentialBackoffRetry")]
#[serde(deny_unknown_fields)]
pub struct ExponentialBackoffRetry {
    #[serde(rename = "type")]
    _kind: MustBe!("exponential_backoff"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[into_proto(map_method = "as_secs_f64")]
    pub delay_s: Duration,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[into_proto(map_method = "as_secs_f64")]
    pub max_delay_s: Duration,
}

fn validate_timeout(timeout: &Duration) -> Result<(), ValidationError> {
    if timeout.as_secs_f64() < 1.0 || timeout.as_secs_f64() > 30.0 {
        return Err(validation_error(
            "invalid_timeout",
            "Timeout must be between 1.0 and 30.0 seconds".to_string(),
        ));
    };
    Ok(())
}
