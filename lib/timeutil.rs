use chrono::{DateTime, Timelike, Utc};
use chrono_tz::{Tz, UTC};
use iso8601_duration::Duration as IsoDuration;

// Note that we reset nanoseconds to 0 to:
// - Ensure that the same time is always returned, regardless of the nanosecond
//   value
// - Enforce per-second granularity

pub fn parse_iso8601(input: &str) -> Option<DateTime<Tz>> {
    let parsed_datetime = DateTime::parse_from_str(input, "%+")
        .map(|t| t.with_nanosecond(0).unwrap().with_timezone(&UTC));

    if parsed_datetime.is_ok() {
        return parsed_datetime.ok();
    }

    let parsed_duration = IsoDuration::parse(input);
    let Ok(duration) = parsed_duration else {
        return None;
    };

    // convert IsoDuration into chrono::Duration
    let duration = chrono::Duration::milliseconds(
        ((duration.year * 60. * 60. * 24. * 30. * 12.
            + duration.month * 60. * 60. * 24. * 30.
            + duration.day * 60. * 60. * 24.
            + duration.hour * 60. * 60.
            + duration.minute * 60.
            + duration.second)
            * 1000.) as i64,
    );
    Some(
        Utc::now()
            .checked_add_signed(duration)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            .with_timezone(&UTC),
    )
}

pub fn to_iso8601(input: &DateTime<Tz>) -> String {
    input.format("%+").to_string()
}

pub mod iso8601_dateformat_serde {
    use chrono::DateTime;
    use chrono_tz::Tz;
    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::parse_iso8601;

    pub fn serialize<S>(
        input: &DateTime<Tz>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", input.format("%+"));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<Tz>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = parse_iso8601(&s).ok_or_else(|| {
            serde::de::Error::custom(
                "Invalid datetime format. Only ISO-8601 is allowed.",
            )
        })?;
        Ok(dt)
    }
}

pub mod iso8601_dateformat_vec_serde {
    use chrono::DateTime;
    use chrono_tz::Tz;
    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::{parse_iso8601, to_iso8601};

    pub fn serialize<S>(
        timepoints: &Vec<DateTime<Tz>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(timepoints.len()))?;
        for t in timepoints {
            seq.serialize_element(&to_iso8601(t))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Vec<DateTime<Tz>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO Can be refactored to use the iso8601_dateformat_serde
        let vec_de = Vec::<String>::deserialize(deserializer)?;
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
        Ok(items)
    }
}

#[test]
fn test_iso8601_duration_parsing() {
    let input1 = "PT5M";
    let result = parse_iso8601(input1);
    let now = Utc::now().with_nanosecond(0).unwrap().with_timezone(&UTC);
    assert!(result.is_some());
    assert_eq!(5, (result.unwrap() - now).num_minutes());
}

#[test]
fn test_iso8601_negative_duration_parsing() {
    let input = "PT-5M";
    let result = parse_iso8601(input);
    let now = Utc::now().with_nanosecond(0).unwrap().with_timezone(&UTC);
    assert!(result.is_some());
    assert_eq!(-5, (result.unwrap() - now).num_minutes());
}

#[test]
fn test_iso8601_parsing() {
    // no nanoseconds
    let input1 = "2023-03-05T21:27:32Z";
    let result = parse_iso8601(input1).unwrap();

    let parsed_datetime = DateTime::parse_from_str(input1, "%+")
        .map(|t| t.with_nanosecond(0).unwrap().with_timezone(&UTC))
        .unwrap();
    assert_eq!(parsed_datetime, result);
    // with nanoseconds trimmed
    let input1 = "2023-03-05T21:27:32.424Z";
    let result = parse_iso8601(input1).unwrap();
    assert_eq!(parsed_datetime, result);
}
