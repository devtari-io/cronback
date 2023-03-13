use chrono_tz::Tz;
use validator::ValidationError;

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
                "Timezone unrecognized '{cron_timezone}'. A valid IANA timezone string is required",
            )
        ));
    };
    Ok(())
}
