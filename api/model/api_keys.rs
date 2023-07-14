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
}

#[derive(Serialize)]
pub struct CreateAPIKeyResponse {
    pub key: String,
}

#[derive(Serialize)]
pub struct ListKeysItem {
    pub id: String,
    pub name: String,
}
