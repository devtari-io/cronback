use chrono::{DateTime, FixedOffset, TimeZone, Timelike, Utc};
use iso8601_duration::Duration as IsoDuration;

pub fn parse_utc_from_rfc3339(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
        .unwrap()
        .with_timezone(&Utc)
}

// Note that we reset nanoseconds to 0 to:
// Left only for historical reasons
// - Ensure that the same time is always returned, regardless of the nanosecond
//   value
// - Enforce per-second granularity
pub fn parse_iso8601_and_duration(
    input: &str,
) -> Option<DateTime<FixedOffset>> {
    let parsed_datetime = DateTime::parse_from_rfc3339(input)
        .map(|t| t.with_nanosecond(0).unwrap());

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
    // convert Utc::now() into DateTime<FixedOffset>
    let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
    Some(
        now.checked_add_signed(duration)
            .unwrap()
            .with_nanosecond(0)
            .unwrap(),
    )
}

// Only use when the input is trusted to be correct ISO8601
pub fn parse_iso8601_unsafe(input: &str) -> DateTime<FixedOffset> {
    parse_iso8601_and_duration(input).unwrap()
}

pub fn to_rfc3339<T>(input: &DateTime<T>) -> String
where
    T: TimeZone,
    <T as TimeZone>::Offset: std::fmt::Display,
{
    input.to_rfc3339_opts(chrono::SecondsFormat::Secs, /* use_z */ true)
}

pub fn default_timezone() -> String {
    "Etc/UTC".to_string()
}

pub mod iso8601_dateformat_serde {
    use chrono::{DateTime, FixedOffset};
    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::parse_iso8601_and_duration;

    pub fn serialize<S>(
        input: &DateTime<FixedOffset>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&input.to_rfc3339())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = parse_iso8601_and_duration(&s).ok_or_else(|| {
            serde::de::Error::custom(
                "Invalid datetime format. Only ISO-8601 is allowed.",
            )
        })?;
        Ok(dt)
    }
}

pub mod iso8601_dateformat_vec_serde {
    use chrono::{DateTime, FixedOffset};
    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::{parse_iso8601_and_duration, to_rfc3339};

    pub fn serialize<S>(
        timepoints: &Vec<DateTime<FixedOffset>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(timepoints.len()))?;
        for t in timepoints {
            seq.serialize_element(&to_rfc3339(t))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Vec<DateTime<FixedOffset>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO Can be refactored to use the iso8601_dateformat_serde
        let vec_de = Vec::<String>::deserialize(deserializer)?;
        let items: Result<Vec<DateTime<FixedOffset>>, D::Error> = vec_de
            .into_iter()
            .map(|x| {
                parse_iso8601_and_duration(&x).ok_or_else(|| {
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
    let result = parse_iso8601_and_duration(input1);
    let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
    let now = now.with_nanosecond(0).unwrap();
    assert!(result.is_some());
    assert_eq!(5, (result.unwrap() - now).num_minutes());
}

#[test]
fn test_iso8601_negative_duration_parsing() {
    let input = "PT-5M";
    let result = parse_iso8601_and_duration(input);
    let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
    let now = now.with_nanosecond(0).unwrap();
    assert!(result.is_some());
    assert_eq!(-5, (result.unwrap() - now).num_minutes());
}

#[test]
fn test_iso8601_parsing() {
    // no nanoseconds
    let input1 = "2023-03-05T21:27:32Z";
    let result = parse_iso8601_and_duration(input1).unwrap();

    let parsed_datetime = DateTime::parse_from_rfc3339(input1)
        .map(|t| t.with_nanosecond(0).unwrap())
        .unwrap();
    assert_eq!(parsed_datetime, result);
    // with nanoseconds trimmed
    let input1 = "2023-03-05T21:27:32.424Z";
    let result = parse_iso8601_and_duration(input1).unwrap();
    assert_eq!(parsed_datetime, result);
}
