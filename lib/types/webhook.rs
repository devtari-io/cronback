use std::time::Duration;

use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use validator::Validate;

use crate::validation::validate_webhook_url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate, Eq)]
pub struct Webhook {
    #[serde(rename = "type")]
    // allows an optional "type" field to be passed in. This enables other
    // variants of action to be differentiated.
    pub _kind: MustBe!("webhook"),
    #[validate(required, custom = "validate_webhook_url")]
    pub url: Option<String>,
    pub http_method: HttpMethod,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub timeout_s: std::time::Duration,
    // None means no retry
    pub retry: Option<RetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum RetryConfig {
    SimpleRetry(SimpleRetry),
    ExponentialBackoffRetry(ExponentialBackoffRetry),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleRetry {
    #[serde(rename = "type")]
    pub _kind: MustBe!("simple"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub delay_s: Duration,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExponentialBackoffRetry {
    #[serde(rename = "type")]
    pub _kind: MustBe!("exponential_backoff"),
    pub max_num_attempts: u32,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub delay_s: Duration,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub max_delay_s: Duration,
}

#[cfg(test)]
mod tests {

    use crate::validation::validate_webhook_url;

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
