use std::time::Duration;

use ipext::IpExt;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use thiserror::Error;
use url::Url;
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
    #[validate(required, custom = "validate_webhook_url")]
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

#[derive(Error, Debug)]
enum EmitValidationError {
    #[error("Failed to parse url: {0}")]
    InvalidUrl(String),

    #[error(
        "Unsupported url scheme: {0}. Only 'http' and 'https' are supported"
    )]
    UnsupportedScheme(String),

    #[error("Failed to resolve ip of url '{0}'")]
    Dns(String),

    #[error("Domain resolves to non-routable public IP: {0}")]
    NonRoutableIp(String),
}

impl From<EmitValidationError> for ValidationError {
    fn from(value: EmitValidationError) -> Self {
        validation_error("EMIT_VALIDATION_FAILED", value.to_string())
    }
}

fn validate_endpoint_scheme(scheme: &str) -> Result<(), EmitValidationError> {
    if scheme == "http" || scheme == "https" {
        Ok(())
    } else {
        Err(EmitValidationError::UnsupportedScheme(scheme.to_string()))
    }
}

fn validate_endpoint_url_public_ip(
    url: &Url,
) -> Result<(), EmitValidationError> {
    if std::env::var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION").is_ok() {
        return Ok(());
    }
    // This function does the DNS resolution. Unfortunately, it's synchronous.
    let addrs = url
        // TODO: Replace with non-blocking nameservice lookup
        .socket_addrs(|| None)
        .map_err(|_| EmitValidationError::Dns(url.to_string()))?;

    // To error on the safe side, a hostname is valid if ALL its IPs are
    // publicly addressable.
    for addr in addrs {
        if !IpExt::is_global(&addr.ip()) {
            return Err(EmitValidationError::NonRoutableIp(
                addr.ip().to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_webhook_url(url_string: &str) -> Result<(), ValidationError> {
    let url = Url::parse(url_string)
        .map_err(|e| EmitValidationError::InvalidUrl(e.to_string()))?;
    validate_endpoint_scheme(url.scheme())?;
    validate_endpoint_url_public_ip(&url)?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::validate_webhook_url;

    #[test]
    fn valid_urls() {
        let urls = vec![
            "https://google.com/url",
            "https://example.com:3030/url",
            "https://1.1.1.1/url",
            "http://[2606:4700:4700::1111]/another_url/path",
            "http://[2606:4700:4700::1111]:5050/another_url/path",
            "http://user:pass@google.com/another_url/path",
        ];

        for url in urls {
            let result = validate_webhook_url(url);
            assert!(
                matches!(result, Ok(())),
                "URL: {}, result: {:?}",
                url,
                result,
            );
        }
    }

    #[test]
    fn invalid_urls() {
        let urls = vec![
            // Private IPs
            "https://10.0.10.1",
            "https://192.168.1.1",
            "https://[::1]:80",
            // Non-http url
            "ftp://google.com",
            // Lookback address
            "https://localhost/url",
            // Scheme-less
            "google.com/url",
            // Unparsable URL
            "http---@goog.com",
            // Non-existent domains
            "https://ppqqzonlnp.io/url/url",
        ];

        for url in urls {
            let result = validate_webhook_url(url);
            assert!(
                matches!(result, Err(_)),
                "URL: {}, result: {:?}",
                url,
                result
            );
        }
    }
}
