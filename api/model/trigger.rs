use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(tag = "object")]
#[serde(deny_unknown_fields)]
#[serde(default)]
#[validate(schema(
    function = "validate_timepoints",
    skip_on_field_errors = false,
))]
pub(crate) struct Trigger {
    #[serde(skip_deserializing)]
    pub id: String,
    #[validate(length(
        min = 2,
        max = 1000,
        message = "name must be between 2 and 1000 characters if set"
    ))]
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reference_id: Option<String>,
    #[validate(length(
        min = 0,
        max = 1048576,
        message = "Payload must be under 1MiB"
    ))]
    pub payload: Option<String>,
    #[validate(length(
        max = 30,
        message = "Max number of headers reached (>=30)"
    ))]
    pub headers: HashMap<String, String>,
    pub content_type: String,
    #[validate(custom = "validate_cron")]
    pub cron: Option<String>,
    #[validate(custom = "validate_timezone")]
    pub cron_timezone: String,
    pub cron_events_limit: u64,
    #[validate(
        length(
            min = 1,
            max = 5000,
            message = "Reached maximum number of run_at events in the same trigger"
        ),
        custom = "validate_run_at"
    )]
    #[serde(with = "iso8601_dateformat")]
    pub run_at: Option<Vec<DateTime<Tz>>>,
    #[validate(range(
        min = 1.0,
        max = 30.0,
        message = "Timeout must be between 1 and 30 seconds"
    ))]
    pub timeout_s: f64,
    #[serde(skip_deserializing)]
    pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    Active,
    Expired,
    Canceled,
    Paused,
}

impl Default for Status {
    fn default() -> Self {
        Self::Active
    }
}

// fn from_request(
impl Default for Trigger {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: None,
            reference_id: None,
            created_at: Utc::now(),
            payload: None,
            headers: Default::default(),
            content_type: "application/json; charset=utf-8".to_string(),
            cron: None,
            cron_timezone: "Etc/UTC".to_string(),
            // 0 means no limit
            cron_events_limit: 0,
            run_at: None,
            timeout_s: 5.0,
            status: Default::default(),
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
                format!("Duplicate value '{}'", timepoint).into(),
            ));
        } else {
            ts.insert(timepoint);
        }
    }
    Ok(())
}

fn validate_timezone(cron_timezone: &String) -> Result<(), ValidationError> {
    // validate timezone
    let tz: Result<Tz, _> = cron_timezone.parse();
    if tz.is_err() {
        return Err(validation_error(
            "unrecognized_cron_timezone",
            format!(
                "Timezone unrecognized '{}'. A valid IANA timezone string is required",
                cron_timezone
            )
        ));
    };
    Ok(())
}

fn validate_timepoints(trigger: &Trigger) -> Result<(), ValidationError> {
    if trigger.cron.is_none()
        && (trigger.run_at.is_none()
            || trigger.run_at.as_ref().unwrap().len() == 0)
    {
        return Err(validation_error(
            "missing_cron_or_run_at",
            format!("Missing cron_timezone or run_at."),
        ));
    }
    if trigger.cron.is_some()
        && (trigger.run_at.is_some()
            && trigger.run_at.as_ref().unwrap().len() > 0)
    {
        return Err(validation_error(
            "cron_and_run_at_mutually_exclusive",
            format!("You cannot set both cron and run_at."),
        ));
    }

    Ok(())
}

fn validation_error(code: &'static str, message: String) -> ValidationError {
    let mut validation_e = ValidationError::new(code);
    validation_e.message = Some(message.into());
    validation_e
}

mod iso8601_dateformat {
    use chrono::DateTime;
    use chrono_tz::Tz;
    use serde::{self, Deserialize, Deserializer, Serializer};
    use shared::timeutil::parse_iso8601;
    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(
        date: &Option<Vec<DateTime<Tz>>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            | None => serializer.serialize_none(),
            | Some(d) => {
                let mut vec = Vec::new();
                for date in d {
                    let s = format!("{}", date.format("%+"));
                    vec.push(s);
                }
                serializer.serialize_some(&vec)
            }
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<Vec<DateTime<Tz>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec_de = Option::<Vec<String>>::deserialize(deserializer)?;
        if vec_de.is_none() {
            return Ok(None);
        }
        let vec_de = vec_de.unwrap();
        let items: Result<Vec<DateTime<Tz>>, D::Error> = vec_de
            .into_iter()
            .map(|x| {
                parse_iso8601(&x).ok_or_else(|| {
                    serde::de::Error::custom(
                        "Invalid datetime format. Only ISO-8601 is allowed.",
                    )
                })
            })
            .collect();
        let items = items?;
        Ok(Some(items))
        //items
    }
}
