use dto::{FromProto, IntoProto};
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

use crate::types::Webhook;

#[derive(
    Debug,
    IntoProto,
    FromProto,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    FromJsonQueryResult,
)]
#[proto(target = "proto::common::Action", non_exhaustive)]
pub enum Action {
    Webhook(Webhook),
}
