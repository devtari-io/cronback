use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::model_util::{generate_model_id, generate_owner_id};

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

impl Display for OwnerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<OwnerId> for String {
    fn from(value: OwnerId) -> Self {
        value.0
    }
}

impl From<String> for OwnerId {
    fn from(value: String) -> Self {
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

impl Display for TriggerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<TriggerId> for String {
    fn from(value: TriggerId) -> Self {
        value.0
    }
}

impl From<String> for TriggerId {
    fn from(value: String) -> Self {
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

impl From<EventId> for String {
    fn from(value: EventId) -> Self {
        value.0
    }
}

impl From<String> for EventId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
)]
#[serde(transparent)]
pub struct CellId(pub u64);

impl CellId {
    pub fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Display for CellId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CellId> for u64 {
    fn from(value: CellId) -> Self {
        value.0
    }
}

impl From<u64> for CellId {
    fn from(value: u64) -> Self {
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
)]
#[serde(transparent)]
pub struct InvocationId(pub String);
impl InvocationId {
    pub fn new(OwnerId(owner): &OwnerId) -> Self {
        Self(generate_model_id("inv", owner))
    }
    pub fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for InvocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<InvocationId> for String {
    fn from(value: InvocationId) -> Self {
        value.0
    }
}

impl From<String> for InvocationId {
    fn from(value: String) -> Self {
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

impl Display for AttemptLogId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<AttemptLogId> for String {
    fn from(value: AttemptLogId) -> Self {
        value.0
    }
}

impl From<String> for AttemptLogId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
