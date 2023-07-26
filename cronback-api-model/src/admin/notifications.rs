use std::collections::HashMap;

#[cfg(feature = "dto")]
use dto::{FromProto, IntoProto};
use monostate::MustBe;
use serde::{Deserialize, Serialize};
#[cfg(feature = "validation")]
use validator::Validate;

#[cfg(feature = "validation")]
use crate::validation_util::validation_error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::notifications::ProjectNotificationSettings")
)]
#[cfg_attr(
    feature = "validation",
    derive(Validate),
    validate(schema(
        function = "validate_settings",
        skip_on_field_errors = false
    ))
)]
#[serde(deny_unknown_fields)]
pub struct NotificationSettings {
    #[cfg_attr(feature = "validation", validate)]
    pub subscriptions: Vec<NotificationSubscriptionConfig>,
    #[cfg_attr(feature = "validation", validate)]
    pub channels: HashMap<String, NotificationChannel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::notifications::NotificationSubscriptionConfig")
)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[serde(deny_unknown_fields)]
pub struct NotificationSubscriptionConfig {
    #[cfg_attr(feature = "validation", validate(length(max = 20)))]
    pub channel_names: Vec<String>,
    #[cfg_attr(feature = "dto", proto(required))]
    #[cfg_attr(feature = "validation", validate)]
    pub subscription: Subscription,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(
        target = "proto::notifications::NotificationChannel",
        oneof = "channel"
    )
)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum NotificationChannel {
    Email(EmailNotification),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(
        target = "proto::notifications::NotificationSubscription",
        oneof = "subscription"
    )
)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Subscription {
    OnRunFailure(OnRunFailure),
}

// Channel configs

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::notifications::Email")
)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct EmailNotification {
    #[serde(rename = "type")]
    _kind: MustBe!("email"),
    #[cfg_attr(feature = "validation", validate(email))]
    pub address: String,
    pub verified: bool,
}

// Subscription configs

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "dto",
    derive(IntoProto, FromProto),
    proto(target = "proto::notifications::OnRunFailure")
)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[serde(deny_unknown_fields)]
pub struct OnRunFailure {
    #[serde(rename = "type")]
    _kind: MustBe!("on_run_failure"),
}

#[cfg(feature = "validation")]
impl Validate for Subscription {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | Subscription::OnRunFailure(o) => o.validate(),
        }
    }
}

#[cfg(feature = "validation")]
impl Validate for NotificationChannel {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            | NotificationChannel::Email(e) => e.validate(),
        }
    }
}

#[cfg(feature = "validation")]
fn validate_settings(
    settings: &NotificationSettings,
) -> Result<(), validator::ValidationError> {
    // Validate that any channel referenced in a subscription actually exists.

    for sub in &settings.subscriptions {
        for channel in &sub.channel_names {
            if !settings.channels.contains_key(channel) {
                return Err(validation_error(
                    "invalid_channel_name",
                    format!(
                        "Channel name '{}' is not configured in channel list",
                        channel
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_valid_settings() -> anyhow::Result<()> {
        let email = EmailNotification {
            _kind: Default::default(),
            address: "test@gmail.com".to_string(),
            verified: true,
        };
        let mut channels = HashMap::new();
        channels.insert("email".to_string(), NotificationChannel::Email(email));
        let setting = NotificationSettings {
            channels,
            subscriptions: vec![NotificationSubscriptionConfig {
                channel_names: vec!["email".to_string()],
                subscription: Subscription::OnRunFailure(OnRunFailure {
                    _kind: Default::default(),
                }),
            }],
        };

        setting.validate()?;
        Ok(())
    }

    #[test]
    fn test_invalid_email() -> anyhow::Result<()> {
        let email = EmailNotification {
            _kind: Default::default(),
            address: "wrong_email".to_string(),
            verified: false,
        };
        let mut channels = HashMap::new();
        channels.insert("email".to_string(), NotificationChannel::Email(email));
        let setting = NotificationSettings {
            channels,
            subscriptions: vec![],
        };

        let validated = setting.validate();

        assert!(validated.is_err());
        assert_eq!(
            validated.unwrap_err().to_string(),
            "channels[0].address: Validation error: email [{\"value\": \
             String(\"wrong_email\")}]"
                .to_string()
        );

        Ok(())
    }

    #[test]
    fn test_invalid_channel() {
        let email = EmailNotification {
            _kind: Default::default(),
            address: "test@gmail.com".to_string(),
            verified: false,
        };
        let mut channels = HashMap::new();
        channels.insert("email".to_string(), NotificationChannel::Email(email));
        let setting = NotificationSettings {
            channels,
            subscriptions: vec![NotificationSubscriptionConfig {
                channel_names: vec![
                    "email".to_string(),
                    "wrong_channel".to_string(),
                ],
                subscription: Subscription::OnRunFailure(OnRunFailure {
                    _kind: Default::default(),
                }),
            }],
        };

        let validated = setting.validate();

        assert!(validated.is_err());
        assert_eq!(
            validated.unwrap_err().to_string(),
            "__all__: Channel name 'wrong_channel' is not configured in \
             channel list"
                .to_string()
        );
    }
}
