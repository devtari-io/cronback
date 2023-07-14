#![cfg(not(feature = "dto"))]

use derive_more::{Deref, Display, From, Into};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    From,
    Into,
    Display,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Hash,
    Deref,
)]
pub struct AttemptLogId(String);

#[derive(
    Debug,
    From,
    Into,
    Display,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Hash,
    Deref,
)]
pub struct RunId(String);

#[derive(
    Debug,
    From,
    Into,
    Display,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Hash,
    Deref,
)]
pub struct TriggerId(String);
