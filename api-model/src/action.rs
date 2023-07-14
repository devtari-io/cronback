#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use super::Webhook;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "client", non_exhaustive)]
/// non_exhaustive because proto doesn't have Event yet.
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::common::Action", non_exhaustive)
)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Action {
    #[cfg_attr(feature = "dto", proto(skip))]
    Event(Event),
    Webhook(Webhook),
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", serde(deny_unknown_fields))]
pub struct Event {
    #[serde(rename = "type")]
    _kind: MustBe!("event"),
    event: String,
}

#[cfg(feature = "validation")]
use validator::Validate;
#[cfg(feature = "validation")]
/// --- Validators ---
impl Validate for Action {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Action::Webhook(webhook) => webhook.validate(),
            | Action::Event(_) => Ok(()),
        }
    }
}
