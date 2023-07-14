use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

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

    // The raw underlying value.
    fn value(&self) -> &str;

    /// Validate that a model Id is well-formed.
    fn is_valid(&self) -> bool;

    // The timestamp section of the underlying Id
    fn timestamp_ms(&self) -> Option<u64> {
        // extract the shard from the id
        if !self.is_valid() {
            return None;
        }

        let (_, after) = self.value().split_once('_').expect("Id is malformed");
        let ulid = &after[4..];
        let ulid = Ulid::from_string(ulid).ok()?;
        Some(ulid.timestamp_ms())
    }
}

/// Indicates that this is a top-level Id (does not follow sharding scheme of
/// another Id)
pub trait RootId: ShardedId {}

pub(crate) fn generate_model_id<T, B>(model_prefix: T, owner: &B) -> String
where
    T: AsRef<str>,
    B: RootId + ShardedId,
{
    format!(
        "{}_{}{}",
        model_prefix.as_ref(),
        owner.shard().encoded(),
        Ulid::new().to_string()
    )
}

pub(crate) fn generate_raw_id<T>(model_prefix: T) -> String
where
    T: AsRef<str>,
{
    // Raw ids are special, but we still prefix the string with the shard
    // identifier even if it's self referential, for consistency that is.
    let new_id = Ulid::new().to_string();
    let mut hasher = DefaultHasher::new();
    new_id.hash(&mut hasher);
    let shard = Shard::from(hasher.finish() % SHARD_COUNT);

    format!("{}_{}{}", model_prefix.as_ref(), shard.encoded(), new_id)
}

/// Define a new model id NewType 
#[rustfmt::skip]
macro_rules! define_model_id {
    (
        #[prefix = $prefix:literal]
        #[no_owner]
        $(#[$m:meta])*
        $type_vis:vis struct $name:ident;
    ) => {
        $(#[$m])*
        #[derive(
            Debug,
            Hash,
            Clone,
            Default,
            ::serde::Serialize,
            ::serde::Deserialize,
            Eq,
            PartialEq,
            PartialOrd,
            Ord,
            ::derive_more::Display,
            ::derive_more::From,
            ::derive_more::Into,
        )]
        #[serde(transparent)]
        $type_vis struct $name(String);

        impl crate::model::RootId for $name {}
        
        impl $name {
            pub fn new() -> Self {
                Self(crate::model::generate_raw_id($prefix))
            }

            pub fn from(value: String) -> Self {
                Self(value)
            }

        }

        impl crate::model::ShardedId for $name {
            fn is_valid(&self) -> bool {
                self.0.starts_with(concat!($prefix, "_"))
            }
            fn value(&self) -> &str {
                &self.0
            }
        }
    };
    (
        #[prefix = $prefix:literal]
        $(#[$m:meta])*
        pub struct $name:ident;
    ) => {
        $(#[$m])*
        #[derive(
            Debug,
            Hash,
            Clone,
            Default,
            ::serde::Serialize,
            ::serde::Deserialize,
            Eq,
            PartialEq,
            PartialOrd,
            Ord,
            ::derive_more::Display,
            ::derive_more::From,
            ::derive_more::Into,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(owner: &impl crate::model::RootId) -> Self {
                Self(crate::model::generate_model_id($prefix, owner))
            }

            pub fn from(value: String) -> Self {
                Self(value)
            }

        }

        impl crate::model::ShardedId for $name {
            fn is_valid(&self) -> bool {
                self.0.starts_with(concat!($prefix, "_"))
            }

            fn value(&self) -> &str {
                &self.0
            }
        }
    };
}

pub(crate) use define_model_id;

#[cfg(test)]
mod tests {
    use super::*;

    define_model_id! {
        #[prefix = "owner"]
        #[no_owner]
        pub struct OwnerId;
    }
    // test that Shard generate encoded string correctly
    #[test]
    fn test_shard_encoding() {
        let shard = Shard::from(123);
        assert_eq!("123", shard.to_string());
        assert_eq!("0123", shard.encoded());
    }

    #[test]
    fn test_model_id_generation() {
        let base = OwnerId::from("owner_049342352".into());
        assert_eq!("0493", base.shard().encoded());
        let id1 = generate_model_id("trig", &base);
        assert!(id1.len() > 4);
        assert!(id1.starts_with("trig_0493"));
    }

    #[test]
    fn test_mode_id_macro() {
        define_model_id! {
            #[prefix = "som"]
            pub struct SomeId;
        }

        let owner = OwnerId::new();

        let id1 = SomeId::new(&owner);
        assert!(id1.is_valid());
        assert!(id1.timestamp_ms().is_some());
        assert!(id1.timestamp_ms().unwrap() > 0);

        assert!(id1.to_string().starts_with("som_"));
        assert!(id1.value().starts_with("som_"));
        assert_eq!(id1.shard(), owner.shard());

        // lexographically ordered
        std::thread::sleep(std::time::Duration::from_millis(2));

        let id2 = SomeId::new(&owner);
        assert!(id2 > id1);
        assert!(id2.timestamp_ms().unwrap() > id1.timestamp_ms().unwrap());
        assert_eq!(id2.shard(), owner.shard());

        // invalid Ids.
        let id1 = SomeId::from("nothing_1234".into());
        assert!(!id1.is_valid());
    }
}
