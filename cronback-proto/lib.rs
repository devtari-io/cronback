pub mod events;
pub mod common {
    tonic::include_proto!("common");

    // pbjson generated code
    include!(concat!(env!("OUT_DIR"), "/common.serde.rs"));

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

    impl PaginationIn {
        // We add this because proto3 doesn't support default values. In tests,
        // we construct the protobuf object directly and we can't rely
        // solely on the default populated in API deserialization.
        pub fn limit(&self) -> usize {
            if self.limit == 0 {
                20
            } else {
                self.limit as usize
            }
        }

        pub fn paginated_query_limit(&self) -> u64 {
            (self.limit() + 1) as u64
        }
    }
}

pub mod scheduler_proto {
    tonic::include_proto!("scheduler_proto");
}

pub mod dispatcher_proto {
    tonic::include_proto!("dispatcher_proto");
}

pub mod trigger_proto {
    tonic::include_proto!("trigger_proto");
    include!(concat!(env!("OUT_DIR"), "/trigger_proto.serde.rs"));
}

pub mod run_proto {
    tonic::include_proto!("run_proto");
}

pub mod attempt_proto {
    tonic::include_proto!("attempt_proto");
    include!(concat!(env!("OUT_DIR"), "/attempt_proto.serde.rs"));
}

pub mod project_srv_proto {
    tonic::include_proto!("project_srv_proto");
    include!(concat!(env!("OUT_DIR"), "/project_srv_proto.serde.rs"));
}

pub const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("file_descriptor");
