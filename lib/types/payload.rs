use std::collections::HashMap;

use dto::{FromProto, IntoProto};
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    FromProto,
    IntoProto,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    FromJsonQueryResult,
)]
#[proto(target = "proto::common::Payload")]
pub struct Payload {
    pub headers: HashMap<String, String>,
    pub content_type: String,
    #[from_proto(map = "string_from_bytes")]
    pub body: String,
}

fn string_from_bytes(input: Vec<u8>) -> String {
    String::from_utf8(input).unwrap()
}
