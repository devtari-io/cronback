use chrono_tz::Tz;
use ipext::IpExt;
use thiserror::Error;
use url::Url;
use validator::ValidationError;

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

pub fn validation_error(
    code: &'static str,
    message: String,
) -> ValidationError {
    let mut validation_e = ValidationError::new(code);
    validation_e.message = Some(message.into());
    validation_e
}

pub fn validate_timezone(
    cron_timezone: &String,
) -> Result<(), ValidationError> {
    // validate timezone
    let tz: Result<Tz, _> = cron_timezone.parse();
    if tz.is_err() {
        return Err(validation_error(
            "unrecognized_cron_timezone",
            format!(
                "Timezone unrecognized '{cron_timezone}'. A valid IANA \
                 timezone string is required",
            ),
        ));
    };
    Ok(())
}

pub fn validate_webhook_url(url_string: &str) -> Result<(), ValidationError> {
    let url = Url::parse(url_string)
        .map_err(|e| WebhookUrlValidationError::InvalidUrl(e.to_string()))?;
    validate_endpoint_scheme(url.scheme())?;
    validate_endpoint_url_public_ip(&url)?;

    Ok(())
}

fn validate_endpoint_url_public_ip(
    url: &Url,
) -> Result<(), WebhookUrlValidationError> {
    if std::env::var("CRONBACK__SKIP_PUBLIC_IP_VALIDATION").is_ok() {
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

impl From<WebhookUrlValidationError> for ValidationError {
    fn from(value: WebhookUrlValidationError) -> Self {
        validation_error("EMIT_VALIDATION_FAILED", value.to_string())
    }
}
