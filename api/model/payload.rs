use std::collections::HashMap;

use dto::{FromProto, IntoProto};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use validator::Validate;

#[serde_as]
#[skip_serializing_none]
#[derive(
    IntoProto,
    FromProto,
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    Validate,
    PartialEq,
)]
#[proto(target = "proto::trigger_proto::Payload")]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    #[validate(length(
        max = 30,
        message = "Max number of headers reached (>=30)"
    ))]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[validate(length(
        min = 0,
        max = 1048576,
        message = "Payload must be under 1MiB"
    ))]
    #[proto(map_from_proto = "string_from_bytes")]
    pub body: String,
}

fn default_content_type() -> String {
    "application/json; charset=utf-8".to_owned()
}

fn string_from_bytes(input: Vec<u8>) -> String {
    String::from_utf8(input).unwrap()
}
