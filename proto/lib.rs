pub mod common {
    tonic::include_proto!("common");

    impl<T> From<chrono::DateTime<T>> for DateTime
    where
        T: chrono::TimeZone,
        <T as chrono::TimeZone>::Offset: std::fmt::Display,
    {
        fn from(value: chrono::DateTime<T>) -> Self {
            Self {
                rfc3339: value.to_rfc3339(),
            }
        }
    }

    impl From<DateTime> for chrono::DateTime<chrono::Utc> {
        fn from(value: DateTime) -> Self {
            chrono::DateTime::parse_from_rfc3339(&value.rfc3339)
                .unwrap()
                .with_timezone(&chrono::Utc)
        }
    }

    impl From<DateTime> for chrono::DateTime<chrono::FixedOffset> {
        fn from(value: DateTime) -> Self {
            chrono::DateTime::parse_from_rfc3339(&value.rfc3339).unwrap()
        }
    }
}

pub mod webhook_proto {
    tonic::include_proto!("webhook_proto");
}

pub mod scheduler_proto {
    tonic::include_proto!("scheduler_proto");
}

pub mod dispatcher_proto {
    tonic::include_proto!("dispatcher_proto");
}

pub mod trigger_proto {
    tonic::include_proto!("trigger_proto");
}

pub mod run_proto {
    tonic::include_proto!("run_proto");
}

pub const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("file_descriptor");
