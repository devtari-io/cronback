use std::time::Duration;

use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use validator::{Validate, ValidationError};

use crate::validation::validation_error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
#[serde(deny_unknown_fields)]
pub enum HttpMethod {
    DELETE,
    GET,
    HEAD,
    PATCH,
    POST,
    PUT,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    #[serde(rename = "type")]
    // allows an optional "type" field to be passed in. This enables other
    // variants of emit to be differentiated.
    pub _kind: MustBe!("webhook"),
    // TODO validate as url
    #[validate(required)]
    pub url: Option<String>,
    pub http_method: HttpMethod,
    #[validate(custom = "validate_timeout")]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub timeout_s: std::time::Duration,
    // None means no retry
    pub retry: Option<RetryConfig>,
}

impl Default for Webhook {
    fn default() -> Self {
        Self {
            _kind: Default::default(),
            url: None,
            http_method: HttpMethod::POST,
            timeout_s: Duration::from_secs(5),
            retry: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum RetryConfig {
    SimpleRetry(SimpleRetry),
    ExponentialBackoffRetry(ExponentialBackoffRetry),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct SimpleRetry {
    #[serde(rename = "type")]
    pub _kind: MustBe!("simple"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
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
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExponentialBackoffRetry {
    #[serde(rename = "type")]
    pub _kind: MustBe!("exponential_backoff"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub delay_s: Duration,
    #[serde_as(as = "DurationSecondsWithFrac")]
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
