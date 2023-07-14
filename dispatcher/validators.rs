use ipext::IpExt;
use lib::types::Webhook;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Missing")]
    MissingUrl,

    #[error("Failed to parse url: {0}")]
    InvalidUrl(String),

    #[error("Unsupported url schema: {0}")]
    UnsupportedScheme(String),

    #[error("Failed to resolve ip of url '{0}'")]
    Dns(String),

    #[error("Domain resolves to non-routable IP: {0}")]
    NonRoutableIp(String),
}

#[allow(dead_code)]
fn validate_endpoint_scheme(scheme: &str) -> Result<(), ValidationError> {
    if scheme == "http" || scheme == "https" {
        Ok(())
    } else {
        Err(ValidationError::UnsupportedScheme(scheme.to_string()))
    }
}

#[allow(dead_code)]
fn validate_endpoint_url_public_ip(url: &Url) -> Result<(), ValidationError> {
    // This function does the DNS resolution. Unfortunately, it's synchronous.
    let addrs = url
        // TODO: Replace with non-blocking nameservice lookup
        .socket_addrs(|| None)
        .map_err(|_| ValidationError::Dns(url.to_string()))?;

    // To error on the safe side, a hostname is valid if ALL its IPs are
    // publicly addressable.
    for addr in addrs {
        if !IpExt::is_global(&addr.ip()) {
            return Err(ValidationError::NonRoutableIp(addr.ip().to_string()));
        }
    }
    Ok(())
}

pub(crate) fn validate_webhook(
    webhook: &Webhook,
) -> Result<(), ValidationError> {
    let url_string = webhook.url.as_ref().ok_or(ValidationError::MissingUrl)?;

    let url = Url::parse(url_string)
        .map_err(|e| ValidationError::InvalidUrl(e.to_string()))?;
    validate_endpoint_scheme(url.scheme())?;
    validate_endpoint_url_public_ip(&url)?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use lib::types::{HttpMethod, Webhook};

    use super::validate_webhook;

    fn build_webhook_from_url(url: &str) -> Webhook {
        Webhook {
            _kind: Default::default(),
            url: Some(url.to_string()),
            http_method: HttpMethod::GET,
            timeout_s: Duration::from_secs(5),
            retry: None,
        }
    }

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
            let result = validate_webhook(&build_webhook_from_url(url));
            assert!(
                matches!(
                    validate_webhook(&build_webhook_from_url(url)),
                    Ok(())
                ),
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
            let result = validate_webhook(&build_webhook_from_url(url));
            assert!(
                matches!(result, Err(_)),
                "URL: {}, result: {:?}",
                url,
                result
            );
        }
    }
}
