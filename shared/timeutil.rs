use chrono::{DateTime, Duration, Timelike, Utc};
use chrono_tz::{Tz, UTC};
use iso8601_duration::Duration as IsoDuration;

// Note that we reset nanoseconds to 0 to:
// - Ensure that the same time is always returned, regardless of the nanosecond value
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

    Some(
        Utc::now()
            .checked_add_signed(Duration::from_std(duration.to_std()).unwrap())
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            .with_timezone(&UTC),
    )
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
