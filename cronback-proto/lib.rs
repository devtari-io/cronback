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

pub mod scheduler_svc {
    tonic::include_proto!("scheduler_svc");
}

pub mod dispatcher_svc {
    tonic::include_proto!("dispatcher_svc");
}

pub mod triggers {
    tonic::include_proto!("triggers");
    include!(concat!(env!("OUT_DIR"), "/triggers.serde.rs"));
}

pub mod runs {
    tonic::include_proto!("runs");
}

pub mod attempts {
    tonic::include_proto!("attempts");
    include!(concat!(env!("OUT_DIR"), "/attempts.serde.rs"));
}

pub mod metadata_svc {
    tonic::include_proto!("metadata_svc");
}

pub mod projects {
    tonic::include_proto!("projects");
    include!(concat!(env!("OUT_DIR"), "/projects.serde.rs"));
}

pub mod notifications {
    tonic::include_proto!("notifications");
}

pub const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("file_descriptor");
