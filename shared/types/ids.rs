use serde::{Deserialize, Serialize};

use crate::model_util::{generate_model_id, generate_owner_id};

use derive_more::{Display, From, Into};

#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct OwnerId(pub String);

impl OwnerId {
    pub fn new() -> Self {
        Self(generate_owner_id("acc"))
    }
    pub fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(
    Debug,
    Hash,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct TriggerId(pub String);
impl TriggerId {
    pub fn new(OwnerId(owner): &OwnerId) -> Self {
        Self(generate_model_id("trig", owner))
    }
    pub fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct EventId(pub String);
impl EventId {
    pub fn new(OwnerId(owner): &OwnerId) -> Self {
        Self(generate_model_id("evt", owner))
    }
    pub fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct CellId(pub u64);

impl CellId {
    pub fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(
    Debug,
    Hash,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct InvocationId(pub String);
impl InvocationId {
    pub fn new(OwnerId(owner): &OwnerId) -> Self {
        Self(generate_model_id("inv", owner))
    }
}

#[derive(
    Debug,
    Hash,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Display,
    From,
    Into,
)]
#[serde(transparent)]
pub struct AttemptLogId(pub String);
impl AttemptLogId {
    pub fn new(OwnerId(owner): &OwnerId) -> Self {
        Self(generate_model_id("att", owner))
    }
    pub fn from(value: String) -> Self {
        Self(value)
    }
}
