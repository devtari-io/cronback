use dto_helpers::IntoProto;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use validator::Validate;

use super::Webhook;

#[derive(IntoProto, Debug, Clone, Serialize, Deserialize, PartialEq)]
/// non_exhaustive because proto doesn't have Event yet.
#[into_proto(into = "proto::trigger_proto::Action", non_exhaustive)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Action {
    #[into_proto(skip)]
    Event(Event),
    Webhook(Webhook),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(rename = "type")]
    _kind: MustBe!("event"),
    event: String,
}

/// --- Validators ---
impl Validate for Action {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Action::Webhook(webhook) => webhook.validate(),
            | Action::Event(_) => Ok(()),
        }
    }
}
