use std::collections::HashSet;
use std::str::FromStr;

use chrono::DateTime;
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use dto_helpers::IntoProto;
use lib::timeutil::{self, iso8601_dateformat_vec_serde};
use lib::validation::{validate_timezone, validation_error};
use proto::trigger_proto;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(IntoProto, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[into_proto(into = "trigger_proto::Schedule")]
#[serde(rename_all = "snake_case")]
//#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub(crate) enum Schedule {
    Recurring(Recurring),
    RunAt(RunAt),
}

#[derive(
    IntoProto, Deserialize, Serialize, Debug, Clone, PartialEq, Validate,
)]
#[into_proto(into = "trigger_proto::Recurring")]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub struct Recurring {
    #[validate(custom = "validate_cron", required)]
    #[into_proto(required)]
    pub cron: Option<String>,
    #[validate(custom = "validate_timezone")]
    #[serde(default = "timeutil::default_timezone")]
    pub timezone: String,
    #[validate(range(min = 1))]
    pub limit: Option<u64>,
    #[serde(skip_deserializing)]
    pub remaining: Option<u64>,
}

#[derive(
    IntoProto, Deserialize, Serialize, Debug, Clone, PartialEq, Validate,
)]
#[into_proto(into = "trigger_proto::RunAt")]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RunAt {
    #[validate(
        length(
            min = 1,
            max = 5000,
            message = "must have at least one but with no more than 5000 \
                       timepoints"
        ),
        custom = "validate_run_at"
    )]
    #[serde(with = "iso8601_dateformat_vec_serde")]
    #[into_proto(map_fn = "timeutil::to_iso8601")]
    pub timepoints: Vec<DateTime<Tz>>,
    #[serde(skip_deserializing)]
    pub remaining: u64,
}

impl Validate for Schedule {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Schedule::Recurring(recurring) => recurring.validate(),
            | Schedule::RunAt(run_at) => run_at.validate(),
        }
    }
}

fn validate_cron(cron_pattern: &String) -> Result<(), ValidationError> {
    if CronSchedule::from_str(cron_pattern).is_err() {
        return Err(validation_error(
            "invalid_cron_pattern",
            format!("Invalid cron_pattern '{}'", cron_pattern),
        ));
    }
    Ok(())
}
// Validate that run_at has no duplicates.
fn validate_run_at(run_at: &Vec<DateTime<Tz>>) -> Result<(), ValidationError> {
    let mut ts = HashSet::new();
    for timepoint in run_at {
        if ts.contains(timepoint) {
            // Duplicate found!
            return Err(validation_error(
                "duplicate_run_at_value",
                format!("Duplicate value '{timepoint}'"),
            ));
        } else {
            ts.insert(timepoint);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::*;

    #[test]
    fn validate_run_at() -> Result<()> {
        let run_at = json!(
            {
                "timepoints": [ "PT1M", "PT2M" ]
            }
        );
        let parsed: RunAt = serde_json::from_value(run_at)?;
        parsed.validate()?;
        assert_eq!(2, parsed.timepoints.len());

        // at least one is needed
        let run_at = json!(
            {
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
                "timepoints": [ "PT1M", "PT1M" ]
            }
        );
        let parsed: RunAt = serde_json::from_value(run_at)?;
        let maybe_validated = parsed.validate();
        assert!(maybe_validated.is_err());
        assert!(maybe_validated
            .unwrap_err()
            .to_string()
            .starts_with("timepoints: Duplicate value"));
        Ok(())
    }

    #[test]
    fn validate_recurring() -> Result<()> {
        // valid cron, every minute.
        let recurring = json!(
            {
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
