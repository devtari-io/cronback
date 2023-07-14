use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[serde(deny_unknown_fields)]
pub struct CreateAPIkeyRequest {
    #[cfg_attr(
        feature = "validation",
        validate(length(
            min = 2,
            max = 30,
            message = "name must be between 2 and 30 characters"
        ))
    )]
    pub key_name: String,
    #[serde(flatten)]
    pub metadata: APIKeyMetaData,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAPIKeyResponse {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[serde(flatten)]
    pub metadata: APIKeyMetaData,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct APIKeyMetaData {
    pub creator_user_id: Option<String>,
}
