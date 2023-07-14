use chrono::{DateTime, FixedOffset};
#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
#[cfg(feature = "validation")]
use validator::{Validate, ValidationError};

#[cfg(feature = "validation")]
use crate::validation_util::validation_error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::trigger_proto::Schedule")
)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Schedule {
    Recurring(Recurring),
    RunAt(RunAt),
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::trigger_proto::Recurring")
)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields))]
#[serde(rename_all = "snake_case")]
pub struct Recurring {
    #[serde(rename = "type")]
    _kind: MustBe!("recurring"),
    #[cfg_attr(
        feature = "validation",
        validate(custom = "validate_cron", required)
    )]
    #[cfg_attr(feature = "dto", proto(required))]
    pub cron: Option<String>,
    #[cfg_attr(feature = "validation", validate(custom = "validate_timezone"))]
    #[cfg_attr(feature = "server", serde(default = "default_timezone"))]
    pub timezone: String,
    #[cfg_attr(feature = "validation", validate(range(min = 1)))]
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::trigger_proto::RunAt")
)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields))]
#[serde(rename_all = "snake_case")]
pub struct RunAt {
    #[serde(rename = "type")]
    _kind: MustBe!("timepoints"),
    #[cfg_attr(
        feature = "validation",
        validate(
            length(
                min = 1,
                max = 5000,
                message = "must have at least one but with no more than 5000 \
                           timepoints"
            ),
            custom = "validate_run_at"
        )
    )]
    pub timepoints: Vec<DateTime<FixedOffset>>,
    pub remaining: Option<u64>,
}

#[cfg(feature = "server")]
fn default_timezone() -> String {
    "Etc/UTC".to_string()
}

#[cfg(feature = "validation")]
impl Validate for Schedule {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Schedule::Recurring(recurring) => recurring.validate(),
            | Schedule::RunAt(run_at) => run_at.validate(),
        }
    }
}

#[cfg(feature = "validation")]
fn validate_cron(cron_pattern: &String) -> Result<(), ValidationError> {
    use std::str::FromStr;

    use cron::Schedule as CronSchedule;
    if CronSchedule::from_str(cron_pattern).is_err() {
        return Err(validation_error(
            "invalid_cron_pattern",
            format!("Invalid cron_pattern '{}'", cron_pattern),
        ));
    }
    Ok(())
}

#[cfg(feature = "validation")]
// Validate that run_at has no duplicates.
fn validate_run_at(
    run_at: &Vec<DateTime<FixedOffset>>,
) -> Result<(), ValidationError> {
    use std::collections::HashSet;

    use chrono::Timelike;

    let mut ts = HashSet::new();
    for timepoint in run_at {
        let trimmed = timepoint.with_nanosecond(0).unwrap();
        if ts.contains(&trimmed) {
            // Duplicate found!
            return Err(validation_error(
                "duplicate_run_at_value",
                format!(
                    "'{timepoint}' conflicts with other timepoints on the \
                     list. Note that the precision is limited to seconds."
                ),
            ));
        } else {
            ts.insert(trimmed);
        }
    }
    Ok(())
}

#[cfg(feature = "validation")]
pub fn validate_timezone(
    cron_timezone: &String,
) -> Result<(), ValidationError> {
    // validate timezone
    use chrono_tz::Tz;
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

#[cfg(all(test, feature = "validation"))]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::*;

    #[test]
    fn validate_run_at() -> Result<()> {
        let run_at = json!(
            {
                "type": "timepoints",
                // a minute difference
                "timepoints": [
                    "2023-06-02T12:40:58+03:00",
                    "2023-06-02T12:41:58+03:00"
                ]
            }
        );
        let parsed: RunAt = serde_json::from_value(run_at)?;
        parsed.validate()?;
        assert_eq!(2, parsed.timepoints.len());

        // at least one is needed
        let run_at = json!(
            {
                "type": "timepoints",
                "timepoints": [ ]
            }
        );
        let parsed: RunAt = serde_json::from_value(run_at)?;
        let maybe_validated = parsed.validate();
        assert!(maybe_validated.is_err());
        assert_eq!(
            "timepoints: must have at least one but with no more than 5000 \
             timepoints"
                .to_owned(),
            maybe_validated.unwrap_err().to_string()
        );

        // no duplicates allowed
        let run_at = json!(
            {
                "type": "timepoints",
                "timepoints": [
                    "2023-06-02T12:40:58+03:00",
                    "2023-06-02T12:40:58+03:00"
                ]
            }
        );
        let parsed: RunAt = serde_json::from_value(run_at)?;
        let maybe_validated = parsed.validate();
        assert!(maybe_validated.is_err());
        assert!(maybe_validated
            .unwrap_err()
            .to_string()
            .starts_with("timepoints: "));
        Ok(())
    }

    #[test]
    fn validate_recurring() -> Result<()> {
        // valid cron, every minute.
        let recurring = json!(
            {
                "type": "recurring",
                "cron": "0 * * * * *",
            }
        );
        let parsed: Recurring = serde_json::from_value(recurring)?;
        parsed.validate()?;
        assert_eq!("0 * * * * *".to_owned(), parsed.cron.unwrap());
        assert_eq!("Etc/UTC".to_owned(), parsed.timezone);
        assert!(parsed.limit.is_none());
        assert!(parsed.remaining.is_none());

        // invalid cron
        let recurring = json!(
            {
                "type": "recurring",
                "cron": "0 * invalid",
            }
        );
        let parsed: Recurring = serde_json::from_value(recurring)?;
        let maybe_validated = parsed.validate();
        assert!(maybe_validated.is_err());
        assert_eq!(
            "cron: Invalid cron_pattern '0 * invalid'".to_owned(),
            maybe_validated.unwrap_err().to_string()
        );

        Ok(())
    }
}
