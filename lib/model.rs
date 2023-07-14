use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use derive_more::{Display, From, Into};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use ulid::Ulid;

const SHARD_COUNT: u64 = 1031;

#[derive(Debug, Error)]
pub enum ModelIdError {
    #[error("Malformed Id: {0}")]
    InvalidId(String),
}

// A shorthand for a Result that returns a ModelIdError
impl From<ModelIdError> for tonic::Status {
    fn from(value: ModelIdError) -> Self {
        tonic::Status::invalid_argument(value.to_string())
    }
}

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
    pub fn encoded(&self) -> String {
        format!("{:04}", self.0)
    }
}

pub trait ModelId: Sized + std::fmt::Display + From<String> {
    fn has_valid_prefix(&self) -> bool;
    fn value(&self) -> &str;
    fn validated(self) -> Result<ValidShardedId<Self>, ModelIdError> {
        ValidShardedId::try_from(self)
    }
}

#[derive(Ord, PartialOrd, Debug, Clone, PartialEq, Eq, Display, Serialize)]
pub struct ValidShardedId<T>(T);

impl<T> ValidShardedId<T>
where
    T: ModelId + From<String>,
{
    pub fn try_from(s: T) -> Result<Self, ModelIdError> {
        // validate Id
        if s.has_valid_prefix() {
            // Can also validate the rest of properties of the Id format.
            // Including a future HMAC signature
            Ok(Self(s))
        } else {
            Err(ModelIdError::InvalidId(s.to_string()))
        }
    }

    // Should be used with caution, as it bypasses validation
    pub fn from_string_unsafe(s: String) -> Self {
        Self(T::from(s))
    }

    /// Returns the shard associated with this Id
    pub fn shard(&self) -> Shard {
        // extract the shard from the id
        let (_, after) = self.value().split_once('_').expect("Id is malformed");
        let shard: u64 = after[..4].parse().expect("Id is malformed");
        Shard::from(shard)
    }

    // The timestamp section of the underlying Id
    pub fn timestamp_ms(&self) -> Option<u64> {
        // extract the shard from the id
        let (_, after) = self.value().split_once('_').expect("Id is malformed");
        let ulid = &after[4..];
        let ulid = Ulid::from_string(ulid).ok()?;
        Some(ulid.timestamp_ms())
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'de, T> Deserialize<'de> for ValidShardedId<T>
where
    T: ModelId + From<String>,
{
    fn deserialize<D>(deserializer: D) -> Result<ValidShardedId<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let id = T::from(s);
        id.validated().map_err(serde::de::Error::custom)
    }
}

impl<T: ModelId> std::ops::Deref for ValidShardedId<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ModelId> From<ValidShardedId<T>> for String {
    fn from(value: ValidShardedId<T>) -> Self {
        value.to_string()
    }
}

impl<T: ModelId> From<ValidShardedId<T>> for sea_query::Value {
    fn from(id: ValidShardedId<T>) -> ::sea_query::Value {
        ::sea_query::Value::String(Some(Box::new(id.value().to_owned())))
    }
}

impl<T: ModelId> sea_orm::TryGetable for ValidShardedId<T> {
    fn try_get_by<I: ::sea_orm::ColIdx>(
        res: &::sea_orm::QueryResult,
        index: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        let val = res.try_get_by::<String, _>(index)?;
        let val: T = val.into();

        val.validated().map_err(|e| {
            sea_orm::TryGetError::DbErr(sea_orm::DbErr::TryIntoErr {
                from: "String",
                into: "ValidShardedId",
                source: Box::new(e),
            })
        })
    }
}

impl<T: ModelId> sea_query::ValueType for ValidShardedId<T> {
    fn try_from(
        v: ::sea_query::Value,
    ) -> Result<Self, ::sea_query::ValueTypeErr> {
        match v {
            | ::sea_query::Value::String(Some(x)) => {
                let val: T = (*x).into();
                val.validated().map_err(|_| sea_query::ValueTypeErr)
            }
            | _ => Err(sea_query::ValueTypeErr),
        }
    }

    fn type_name() -> String {
        stringify!($name).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> sea_query::ColumnType {
        sea_query::ColumnType::String(None)
    }
}

impl<T: ModelId> sea_query::Nullable for ValidShardedId<T> {
    fn null() -> ::sea_query::Value {
        ::sea_query::Value::String(None)
    }
}

/// Indicates that this is a top-level Id (does not follow sharding scheme of
/// another Id)
pub trait RootId: ModelId {}

pub(crate) fn generate_model_id<T, B>(
    model_prefix: T,
    owner: &ValidShardedId<B>,
) -> String
where
    T: AsRef<str>,
    B: RootId,
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
macro_rules! define_model_id_base {
    (
        #[prefix = $prefix:literal]
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

        impl $crate::model::ModelId for $name {
            fn has_valid_prefix(&self) -> bool {
                self.0.starts_with(concat!($prefix, "_"))
            }
            fn value(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<$name> for $crate::model::ValidShardedId<$name> {
            type Error = $crate::model::ModelIdError;
            fn try_from(id: $name) -> Result<Self, Self::Error> {
                crate::model::ModelId::validated(id)
            }
        }

        impl From<$name> for ::sea_query::Value {
            fn from(id: $name) -> ::sea_query::Value {
                ::sea_query::Value::String(Some(Box::new(id.0.to_owned())))
            }
        }

        impl ::sea_orm::TryGetable for $name {
            fn try_get_by<I: ::sea_orm::ColIdx>(
                res: &::sea_orm::QueryResult,
                index: I
            ) -> Result<Self, sea_orm::TryGetError> {
                let val = res.try_get_by::<String, _>(index)?;
                Ok(val.into())
            }

        }

        impl ::sea_query::ValueType for $name {
            fn try_from(v: ::sea_query::Value) -> Result<Self, ::sea_query::ValueTypeErr> {
                match v {
                    ::sea_query::Value::String(Some(x)) => Ok((*x).into()),
                    _ => Err(sea_query::ValueTypeErr),
                }
            }

            fn type_name() -> String {
                stringify!($name).to_owned()
            }

            fn array_type() -> sea_orm::sea_query::ArrayType {
                sea_orm::sea_query::ArrayType::String
            }

            fn column_type() -> sea_query::ColumnType {
                sea_query::ColumnType::String(None)
            }
        }

        impl sea_query::Nullable for $name {
            fn null() -> ::sea_query::Value {
                ::sea_query::Value::String(None)
            }
        }

        // Unfortunately we can't implement this generically!
        impl From<$crate::model::ValidShardedId<$name>> for $name {
            fn from(value: $crate::model::ValidShardedId<$name>) -> Self {
                value.into_inner()
            }
        }

    };
}

#[rustfmt::skip]
macro_rules! define_model_id {
    (
        #[prefix = $prefix:literal]
        #[no_owner]
        $(#[$m:meta])*
        $type_vis:vis struct $name:ident;
    ) => {

        $crate::model::define_model_id_base!{
            #[prefix = $prefix]
            $(#[$m])*
            $type_vis struct $name;
        }

        impl $crate::model::RootId for $name {}
        
        impl $name {
            pub fn generate() -> $crate::model::ValidShardedId<Self> {
                $crate::model::ValidShardedId::from_string_unsafe(
                    $crate::model::generate_raw_id($prefix)
                )
            }
        }
    };
    (
        #[prefix = $prefix:literal]
        $(#[$m:meta])*
        $type_vis:vis struct $name:ident;
    ) => {
        $crate::model::define_model_id_base!{
            #[prefix = $prefix]
            $(#[$m])*
            $type_vis struct $name;
        }

        impl $name {
            pub fn generate(owner: &$crate::model::ValidShardedId<impl $crate::model::RootId>) -> $crate::model::ValidShardedId<Self> {
                $crate::model::ValidShardedId::from_string_unsafe(
                    $crate::model::generate_model_id($prefix, owner)
                )
            }

            pub fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

pub(crate) use {define_model_id, define_model_id_base};

#[cfg(test)]
mod tests {
    use anyhow::Result;

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
    fn test_model_id_generation() -> Result<()> {
        let base = ValidShardedId::<OwnerId>::from_string_unsafe(
            "owner_049342352".into(),
        );

        assert_eq!("0493", base.shard().encoded());
        let id1 = generate_model_id("trig", &base);
        assert!(id1.len() > 4);
        assert!(id1.starts_with("trig_0493"));
        Ok(())
    }

    #[test]
    fn test_mode_id_macro() -> Result<()> {
        define_model_id! {
            #[prefix = "som"]
            pub struct SomeId;
        }

        let owner = OwnerId::generate();

        let id1 = SomeId::generate(&owner);
        assert!(id1.timestamp_ms().is_some());
        assert!(id1.timestamp_ms().unwrap() > 0);

        assert!(id1.to_string().starts_with("som_"));
        assert!(id1.value().starts_with("som_"));
        assert_eq!(id1.shard(), owner.shard());

        // lexographically ordered
        std::thread::sleep(std::time::Duration::from_millis(2));

        let id2 = SomeId::generate(&owner);
        assert!(id2 > id1);
        assert!(id2.timestamp_ms().unwrap() > id1.timestamp_ms().unwrap());
        assert_eq!(id2.shard(), owner.shard());

        // invalid Ids.
        let id1 = SomeId::from("nothing_1234".into());
        assert!(id1.validated().is_err());
        Ok(())
    }
}
