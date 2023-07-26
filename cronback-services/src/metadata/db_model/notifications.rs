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
    pub subscriptions: Vec<NotificationSubscriptionConfig>,
    pub channels: HashMap<String, NotificationChannel>,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, FromProto, IntoProto,
)]
#[proto(target = "proto::notifications::NotificationSubscriptionConfig")]
pub struct NotificationSubscriptionConfig {
    pub channel_names: Vec<String>,
    #[proto(required)]
    pub subscription: Subscription,
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
#[proto(
    target = "proto::notifications::NotificationSubscription",
    oneof = "subscription"
)]
pub enum Subscription {
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
