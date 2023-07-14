tonic::include_proto!("events");

// pbjson generated code
include!(concat!(env!("OUT_DIR"), "/events.serde.rs"));

pub use event::Details as Events;
use ulid::Ulid;

use crate::common::ProjectId;

impl Event {
    pub fn new(event: Events) -> Self {
        Self {
            id: Ulid::new().to_string(),
            created_at: Some(chrono::Utc::now().into()),
            project_id: None,
            details: Some(event),
        }
    }

    pub fn from_project(
        project_id: impl Into<ProjectId>,
        event: Events,
    ) -> Self {
        Self {
            id: Ulid::new().to_string(),
            created_at: Some(chrono::Utc::now().into()),
            project_id: Some(project_id.into()),
            details: Some(event),
        }
    }
}
