use std::collections::HashMap;

#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
#[cfg(feature = "validation")]
use validator::Validate;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::common::Payload")
)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(
    feature = "server",
    derive(Default),
    serde(default),
    serde(deny_unknown_fields)
)]
pub struct Payload {
    #[cfg_attr(
        feature = "validation",
        validate(length(
            max = 30,
            message = "Max number of headers reached (>=30)"
        ))
    )]
    pub headers: HashMap<String, String>,
    #[cfg_attr(feature = "server", serde(default = "default_content_type"))]
    pub content_type: String,
    #[cfg_attr(
        feature = "validation",
        validate(length(
            min = 0,
            max = 102400,
            message = "Payload body size limit reached (>=102400)"
        ))
    )]
    #[cfg_attr(feature = "dto", from_proto(map = "string_from_bytes"))]
    pub body: String,
}

#[cfg(feature = "server")]
fn default_content_type() -> String {
    "application/json; charset=utf-8".to_owned()
}

#[cfg(feature = "dto")]
fn string_from_bytes(input: Vec<u8>) -> String {
    String::from_utf8(input).unwrap()
}
