use std::collections::HashMap;

use dto::{FromProto, IntoProto};
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Default,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    FromJsonQueryResult,
    FromProto,
    IntoProto,
)]
#[proto(target = "proto::notifications::ProjectNotificationSettings")]
pub struct NotificationSettings {
    pub default_subscriptions: Vec<NotificationSubscription>,
    pub channels: HashMap<String, NotificationChannel>,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromProto, IntoProto,
)]
#[proto(target = "proto::notifications::NotificationSubscription")]
pub struct NotificationSubscription {
    pub channel_names: Vec<String>,
    #[proto(required)]
    pub event: NotificationEvent,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromProto, IntoProto,
)]
#[proto(
    target = "proto::notifications::NotificationChannel",
    oneof = "channel"
)]
pub enum NotificationChannel {
    Email(EmailNotification),
}

#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromProto, IntoProto,
)]
#[proto(target = "proto::notifications::NotificationEvent", oneof = "event")]
pub enum NotificationEvent {
    OnRunFailure(OnRunFailure),
}

// Channel configs

#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromProto, IntoProto,
)]
#[proto(target = "proto::notifications::Email")]
pub struct EmailNotification {
    pub address: String,
    pub verified: bool,
}

// Subscription configs

#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromProto, IntoProto,
)]
#[proto(target = "proto::notifications::OnRunFailure")]
pub struct OnRunFailure {}
