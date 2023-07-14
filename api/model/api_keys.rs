use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateAPIkeyRequest {
    #[validate(length(
        min = 2,
        max = 30,
        message = "name must be between 2 and 30 characters"
    ))]
    pub key_name: String,
    #[serde(flatten)]
    pub metadata: APIKeyMetaData,
}

#[derive(Serialize)]
pub struct CreateAPIKeyResponse {
    pub key: String,
}

#[derive(Serialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[serde(flatten)]
    pub metadata: APIKeyMetaData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct APIKeyMetaData {
    pub creator_user_id: Option<String>,
}
