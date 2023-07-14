use std::time::Duration;

#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
#[cfg(feature = "validation")]
use ipext::IpExt;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DurationSecondsWithFrac};
#[cfg(feature = "validation")]
use thiserror::Error;
#[cfg(feature = "validation")]
use url::Url;
#[cfg(feature = "validation")]
use validator::{Validate, ValidationError};

#[cfg(feature = "validation")]
use crate::validation_util::validation_error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::webhook_proto::HttpMethod")
)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Delete,
    Get,
    Head,
    Patch,
    Post,
    Put,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::webhook_proto::Webhook")
)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields), serde(default))]
pub struct Webhook {
    // allows an optional "type" field to be passed in. This enables other
    // variants of action to be differentiated.
    #[serde(rename = "type")]
    _kind: MustBe!("webhook"),
    #[cfg_attr(
        feature = "validation",
        validate(required, custom = "validate_webhook_url")
    )]
    #[cfg_attr(feature = "dto", proto(required))]
    pub url: Option<String>,
    pub http_method: HttpMethod,
    #[cfg_attr(feature = "validation", validate(custom = "validate_timeout"))]
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[cfg_attr(
        feature = "dto",
        proto(
            map_into_proto = "std::time::Duration::as_secs_f64",
            map_into_by_ref,
            map_from_proto = "Duration::from_secs_f64"
        )
    )]
    pub timeout_s: std::time::Duration,
    // None means no retry
    pub retry: Option<RetryConfig>,
}

#[cfg(feature = "server")]
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

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::webhook_proto::RetryConfig", oneof = "policy")
)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields))]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum RetryConfig {
    #[cfg_attr(feature = "dto", proto(name = "Simple"))]
    SimpleRetry(SimpleRetry),
    #[cfg_attr(feature = "dto", proto(name = "ExponentialBackoff"))]
    ExponentialBackoffRetry(ExponentialBackoffRetry),
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::webhook_proto::SimpleRetry")
)]
#[cfg_attr(feature = "server", serde(default), serde(deny_unknown_fields))]
pub struct SimpleRetry {
    #[serde(rename = "type")]
    _kind: MustBe!("simple"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[cfg_attr(
        feature = "dto",
        proto(
            map_into_proto = "std::time::Duration::as_secs_f64",
            map_into_by_ref,
            map_from_proto = "Duration::from_secs_f64"
        )
    )]
    pub delay_s: Duration,
}

#[cfg(feature = "server")]
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
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::webhook_proto::ExponentialBackoffRetry")
)]
#[serde(deny_unknown_fields)]
pub struct ExponentialBackoffRetry {
    #[serde(rename = "type")]
    _kind: MustBe!("exponential_backoff"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[cfg_attr(
        feature = "dto",
        proto(
            map_into_proto = "std::time::Duration::as_secs_f64",
            map_into_by_ref,
            map_from_proto = "Duration::from_secs_f64"
        )
    )]
    pub delay_s: Duration,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[cfg_attr(
        feature = "dto",
        proto(
            map_into_proto = "std::time::Duration::as_secs_f64",
            map_into_by_ref,
            map_from_proto = "Duration::from_secs_f64"
        )
    )]
    pub max_delay_s: Duration,
}

#[cfg(feature = "validation")]
fn validate_timeout(timeout: &Duration) -> Result<(), ValidationError> {
    if timeout.as_secs_f64() < 1.0 || timeout.as_secs_f64() > 30.0 {
        return Err(validation_error(
            "invalid_timeout",
            "Timeout must be between 1.0 and 30.0 seconds".to_string(),
        ));
    };
    Ok(())
}

#[cfg(feature = "validation")]
#[derive(Error, Debug)]
enum WebhookUrlValidationError {
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

#[cfg(feature = "validation")]
pub fn validate_webhook_url(url_string: &str) -> Result<(), ValidationError> {
    let url = Url::parse(url_string)
        .map_err(|e| WebhookUrlValidationError::InvalidUrl(e.to_string()))?;
    validate_endpoint_scheme(url.scheme())?;
    validate_endpoint_url_public_ip(&url)?;

    Ok(())
}

#[cfg(feature = "validation")]
fn validate_endpoint_url_public_ip(
    url: &Url,
) -> Result<(), WebhookUrlValidationError> {
    // TODO: Move to a non-global setting.
    if let Ok(val) = std::env::var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION") {
        eprintln!(
            "Skipping public ip validation because  \
             'CRONBACK__SKIP_PUBLIC_IP_VALIDATION' env is set to {val}!"
        );
        return Ok(());
    }
    // This function does the DNS resolution. Unfortunately, it's synchronous.
    let addrs = url
        // TODO: Replace with non-blocking nameservice lookup
        .socket_addrs(|| None)
        .map_err(|_| WebhookUrlValidationError::Dns(url.to_string()))?;

    // To error on the safe side, a hostname is valid if ALL its IPs are
    // publicly addressable.
    for addr in addrs {
        if !IpExt::is_global(&addr.ip()) {
            return Err(WebhookUrlValidationError::NonRoutableIp(
                addr.ip().to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "validation")]
fn validate_endpoint_scheme(
    scheme: &str,
) -> Result<(), WebhookUrlValidationError> {
    if scheme == "http" || scheme == "https" {
        Ok(())
    } else {
        Err(WebhookUrlValidationError::UnsupportedScheme(
            scheme.to_string(),
        ))
    }
}

#[cfg(feature = "validation")]
impl From<WebhookUrlValidationError> for ValidationError {
    fn from(value: WebhookUrlValidationError) -> Self {
        validation_error("EMIT_VALIDATION_FAILED", value.to_string())
    }
}

#[cfg(all(test, feature = "validation"))]
mod tests {

    use super::validate_webhook_url;

    #[test]
    fn valid_urls() {
        // This is a best effort approach to enable validation. This will
        // sporadically fail due to the fact the env vars are shared
        // process-wide.
        // TODO: Replace with a more robust approach
        std::env::remove_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION");
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
        std::env::remove_var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION");
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
