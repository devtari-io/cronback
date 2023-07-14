use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::types::ProjectId;

const SHARD_COUNT: u64 = 1031;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct Shard(pub u64);

impl Shard {
    pub fn from(value: u64) -> Self {
        Self(value)
    }

    pub fn encoded(&self) -> String {
        format!("{:04}", self.0)
    }
}

// An Id in the format of `prefix_{:04}{}`, where The first 4 chars after _ is
// the shard.
pub trait ShardedId: std::fmt::Display {
    /// Returns the shard associated with this Id
    fn shard(&self) -> Shard {
        // extract the shard from the id
        let (_, after) = self.value().split_once('_').expect("Id is malformed");
        let shard: u64 = after[..4].parse().expect("Id is malformed");
        Shard::from(shard)
    }
    fn value(&self) -> &str;
}

pub trait ValidId: std::fmt::Display {
    fn is_valid(&self) -> bool;
}

pub(crate) fn generate_model_id<T>(
    model_prefix: T,
    project: &ProjectId,
) -> String
where
    T: AsRef<str>,
{
    format!(
        "{}_{}{}",
        model_prefix.as_ref(),
        project.shard().encoded(),
        Ulid::new().to_string()
    )
}

pub(crate) fn shard_from_raw_project_id(id: &str) -> Shard {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    Shard::from(hasher.finish() % SHARD_COUNT)
}

pub(crate) fn generate_project_id<T>(model_prefix: T) -> String
where
    T: AsRef<str>,
{
    // Project ids are special, but we still prefix the string with the shard
    // identifier even if it's self referential, for consistency that is.
    let new_id = Ulid::new().to_string();
    let mut hasher = DefaultHasher::new();
    new_id.hash(&mut hasher);
    let shard = Shard::from(hasher.finish() % SHARD_COUNT);

    format!("{}_{}{}", model_prefix.as_ref(), shard.encoded(), new_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id_generation() {
        let id1 = generate_project_id("prj");
        assert!(id1.len() > 4);
        assert!(id1.starts_with("prj_"));

        let project = ProjectId::from("prj_449".into());
        assert_eq!("0971", project.shard().encoded());
        let id1 = generate_model_id("trig", &project);
        assert!(id1.len() > 4);
        assert!(id1.starts_with("trig_0971"));
    }
}
